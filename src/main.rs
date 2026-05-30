use clap::{Parser, Subcommand, ValueEnum};
use nyx::api::input::{normalize_source_input, NormalizedSource};
use nyx::api::learning::{
    append_learning_event, create_learning_event, extract_program_features, learning_log_path,
    mode_default_strategies, program_hash, read_learning_events, select_strategies,
    summarize_strategies, threshold_for_mode, time_saved_metrics, LearningEvent,
    ProgramFeatures, StrategyDecision,
};
use nyx::api::NyxCompiler;
use nyx::llm::{LLMClient, OptimizationSuggestion};
use serde::Serialize;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, System};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Auto,
    Speed,
    Memory,
    Balanced,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Auto => "auto",
            Mode::Speed => "speed",
            Mode::Memory => "memory",
            Mode::Balanced => "balanced",
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "nyx")]
#[command(about = "Nyx Runtime: self-optimizing runtime engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(long, default_value = ".nyx/metrics.jsonl")]
    log_file: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a program with baseline + optimized execution and report speedup
    Run {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = Mode::Auto)]
        mode: Mode,
        #[arg(long, default_value_t = false)]
        no_llm: bool,
    },
    /// Analyze profile and optimization opportunities
    Analyze {
        file: PathBuf,
        #[arg(long, default_value_t = false)]
        no_llm: bool,
    },
    /// Apply optimization passes and report instruction deltas
    Optimize {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = Mode::Auto)]
        mode: Mode,
        #[arg(long, default_value_t = false)]
        no_llm: bool,
    },
    /// Start HTTP API server for remote optimization workflows
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Run { file, mode, no_llm } => run_command(&file, mode, no_llm, &cli.log_file),
        Command::Analyze { file, no_llm } => analyze_command(&file, no_llm, &cli.log_file),
        Command::Optimize { file, mode, no_llm } => {
            optimize_command(&file, mode, no_llm, &cli.log_file)
        }
        Command::Serve { host, port } => serve_command(host, port, cli.log_file.clone()),
    };

    if let Err(err) = result {
        eprintln!("{{\"success\":false,\"error\":\"{}\"}}", err);
        std::process::exit(1);
    }
}

fn serve_command(host: String, port: u16, log_file: PathBuf) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("failed to initialize runtime: {}", err))?;

    runtime.block_on(nyx::api::server::run_server(host, port, log_file))
}

fn run_command(file: &Path, mode: Mode, no_llm: bool, log_file: &Path) -> Result<(), String> {
    let normalized = read_source(file)?;
    let mut compiler = NyxCompiler::new();

    let compile = compiler.compile(&normalized.source);
    if !compile.success {
        return Err(compile
            .error
            .unwrap_or_else(|| "failed to compile input".to_string()));
    }

    let baseline_exec = compiler.execute();
    let baseline_profile = compiler.profile();

    let (llm_status, llm_suggestions) = maybe_collect_llm_suggestions(&baseline_profile, no_llm);
    let (features, decision, learning_file, history) =
        build_strategy_decision(&normalized.source, mode, &llm_suggestions, log_file);
    let historical_event_count = history.len();

    let optimize = compiler.optimize_with_strategies(&decision.selected_strategies);
    let optimized_exec = compiler.execute();
    let baseline_return_value = baseline_exec.return_value;
    let optimized_return_value = optimized_exec.return_value;
    let correctness_verified = baseline_return_value == optimized_return_value;

    let speedup_ratio = if optimized_exec.total_time_us > 0 {
        baseline_exec.total_time_us as f64 / optimized_exec.total_time_us as f64
    } else {
        0.0
    };

    let learning_event = create_learning_event(
        &normalized.source,
        features.clone(),
        llm_suggestions
            .iter()
            .map(|s| s.strategy.clone())
            .collect(),
        &decision,
        optimize.optimizations_applied.clone(),
        speedup_ratio,
        mode.as_str().to_string(),
        baseline_exec.total_time_us,
        optimized_exec.total_time_us,
    );
    if let Err(err) = append_learning_event(&learning_file, &learning_event) {
        eprintln!("learning log write failed: {}", err);
    }

    let mut history_with_current = history;
    history_with_current.push(learning_event.clone());
    let savings = time_saved_metrics(&history_with_current);

    let output = serde_json::json!({
        "success": true,
        "command": "run",
        "input_file": file.to_string_lossy(),
        "input_format": normalized.input_format,
        "source_normalized": normalized.normalization_applied,
        "mode": mode,
        "program_hash": learning_event.program_hash,
        "input_features": features,
        "llm_status": llm_status,
        "execution_time_before_us": baseline_exec.total_time_us,
        "execution_time_after_us": optimized_exec.total_time_us,
        "time_saved_us": learning_event.time_saved_us,
        "total_time_saved_us": savings.total_time_saved_us,
        "time_saved_today_us": savings.time_saved_today_us,
        "speedup_ratio": speedup_ratio,
        "process_memory_bytes": current_process_memory_bytes(),
        "historical_event_count": historical_event_count,
        "reused_history": decision.reused_history,
        "program_cached_strategies": decision.program_cached_strategies,
        "program_cache_hit": decision.program_cache_hit,
        "retained_history_events": decision.retained_history_events,
        "selected_strategies": decision.selected_strategies,
        "strategy_scores": decision.strategy_scores,
        "instruction_count_before": optimize.instructions_before,
        "instruction_count_after": optimize.instructions_after,
        "instructions_removed": optimize.instructions_removed,
        "optimization_decisions": optimize.optimizations_applied,
        "llm_suggestions": llm_suggestions,
        "baseline_return_value": baseline_return_value,
        "optimized_return_value": optimized_return_value,
        "correctness_verified": correctness_verified,
        "return_value": optimized_exec.return_value,
    });

    print_json(&output)?;
    append_metrics_log(log_file, "run", &output)?;
    Ok(())
}

fn analyze_command(file: &Path, no_llm: bool, log_file: &Path) -> Result<(), String> {
    let normalized = read_source(file)?;
    let mut compiler = NyxCompiler::new();

    let compile = compiler.compile(&normalized.source);
    if !compile.success {
        return Err(compile
            .error
            .unwrap_or_else(|| "failed to compile input".to_string()));
    }

    let execute = compiler.execute();
    let profile = compiler.profile();
    let analysis = compiler.analyze();
    let features = extract_program_features(&normalized.source);
    let hash = program_hash(&normalized.source);
    let learning_file = learning_log_path(log_file);
    let history = read_learning_events(&learning_file, 1000);
    let strategy_success = summarize_strategies(&history, 12);
    let savings = time_saved_metrics(&history);

    let (llm_status, llm_suggestions) = maybe_collect_llm_suggestions(&profile, no_llm);

    let output = serde_json::json!({
        "success": true,
        "command": "analyze",
        "input_file": file.to_string_lossy(),
        "input_format": normalized.input_format,
        "source_normalized": normalized.normalization_applied,
        "program_hash": hash,
        "input_features": features,
        "llm_status": llm_status,
        "execution_time_us": execute.total_time_us,
        "total_time_saved_us": savings.total_time_saved_us,
        "time_saved_today_us": savings.time_saved_today_us,
        "process_memory_bytes": current_process_memory_bytes(),
        "hot_instruction_count": execute.hot_instruction_count,
        "profile": profile,
        "analysis": analysis,
        "strategy_success_rates": strategy_success,
        "llm_suggestions": llm_suggestions,
    });

    print_json(&output)?;
    append_metrics_log(log_file, "analyze", &output)?;
    Ok(())
}

fn optimize_command(file: &Path, mode: Mode, no_llm: bool, log_file: &Path) -> Result<(), String> {
    let normalized = read_source(file)?;
    let mut compiler = NyxCompiler::new();

    let compile = compiler.compile(&normalized.source);
    if !compile.success {
        return Err(compile
            .error
            .unwrap_or_else(|| "failed to compile input".to_string()));
    }

    let baseline_exec = compiler.execute();
    let baseline_profile = compiler.profile();

    let (llm_status, llm_suggestions) = maybe_collect_llm_suggestions(&baseline_profile, no_llm);
    let (features, decision, learning_file, history) =
        build_strategy_decision(&normalized.source, mode, &llm_suggestions, log_file);
    let historical_event_count = history.len();

    let optimize = compiler.optimize_with_strategies(&decision.selected_strategies);
    let optimized_exec = compiler.execute();
    let baseline_return_value = baseline_exec.return_value;
    let optimized_return_value = optimized_exec.return_value;
    let correctness_verified = baseline_return_value == optimized_return_value;

    let speedup_ratio = if optimized_exec.total_time_us > 0 {
        baseline_exec.total_time_us as f64 / optimized_exec.total_time_us as f64
    } else {
        0.0
    };

    let learning_event = create_learning_event(
        &normalized.source,
        features.clone(),
        llm_suggestions
            .iter()
            .map(|s| s.strategy.clone())
            .collect(),
        &decision,
        optimize.optimizations_applied.clone(),
        speedup_ratio,
        mode.as_str().to_string(),
        baseline_exec.total_time_us,
        optimized_exec.total_time_us,
    );
    if let Err(err) = append_learning_event(&learning_file, &learning_event) {
        eprintln!("learning log write failed: {}", err);
    }

    let mut history_with_current = history;
    history_with_current.push(learning_event.clone());
    let savings = time_saved_metrics(&history_with_current);

    let output = serde_json::json!({
        "success": true,
        "command": "optimize",
        "input_file": file.to_string_lossy(),
        "input_format": normalized.input_format,
        "source_normalized": normalized.normalization_applied,
        "mode": mode,
        "program_hash": learning_event.program_hash,
        "input_features": features,
        "llm_status": llm_status,
        "execution_time_before_us": baseline_exec.total_time_us,
        "execution_time_after_us": optimized_exec.total_time_us,
        "time_saved_us": learning_event.time_saved_us,
        "total_time_saved_us": savings.total_time_saved_us,
        "time_saved_today_us": savings.time_saved_today_us,
        "speedup_ratio": speedup_ratio,
        "historical_event_count": historical_event_count,
        "reused_history": decision.reused_history,
        "program_cached_strategies": decision.program_cached_strategies,
        "program_cache_hit": decision.program_cache_hit,
        "retained_history_events": decision.retained_history_events,
        "selected_strategies": decision.selected_strategies,
        "strategy_scores": decision.strategy_scores,
        "process_memory_bytes": current_process_memory_bytes(),
        "instruction_count_before": optimize.instructions_before,
        "instruction_count_after": optimize.instructions_after,
        "instructions_removed": optimize.instructions_removed,
        "optimization_decisions": optimize.optimizations_applied,
        "llm_suggestions": llm_suggestions,
        "baseline_return_value": baseline_return_value,
        "optimized_return_value": optimized_return_value,
        "correctness_verified": correctness_verified,
    });

    print_json(&output)?;
    append_metrics_log(log_file, "optimize", &output)?;
    Ok(())
}

fn maybe_collect_llm_suggestions(
    profile: &nyx::api::ProfileResult,
    no_llm: bool,
) -> (String, Vec<OptimizationSuggestion>) {
    if no_llm {
        return ("disabled".to_string(), Vec::new());
    }

    let client = LLMClient::new();
    if !client.is_available() {
        return ("unavailable".to_string(), Vec::new());
    }

    let profile_json = serde_json::to_string_pretty(profile).unwrap_or_else(|_| "{}".to_string());
    match client.analyze_profile(&profile_json) {
        Ok(analysis) => ("connected".to_string(), analysis.suggestions),
        Err(err) => (format!("analysis_failed: {}", err), Vec::new()),
    }
}

fn build_strategy_decision(
    source: &str,
    mode: Mode,
    llm_suggestions: &[OptimizationSuggestion],
    log_file: &Path,
) -> (ProgramFeatures, StrategyDecision, PathBuf, Vec<LearningEvent>) {
    let features = extract_program_features(source);
    let hash = program_hash(source);
    let learning_file = learning_log_path(log_file);
    let history = read_learning_events(&learning_file, 1000);

    let mode_label = mode.as_str();
    let defaults = mode_default_strategies(mode_label);
    let llm_inputs: Vec<(String, f32)> = llm_suggestions
        .iter()
        .map(|s| (s.strategy.clone(), s.confidence))
        .collect();

    let decision = select_strategies(
        &defaults,
        &llm_inputs,
        &features,
        &history,
        threshold_for_mode(mode_label),
        mode_label,
        &hash,
    );

    (features, decision, learning_file, history)
}

fn read_source(file: &Path) -> Result<NormalizedSource, String> {
    let source = fs::read_to_string(file)
        .map_err(|err| format!("failed to read {}: {}", file.display(), err))?;
    Ok(normalize_source_input(&source))
}

fn print_json(output: &Value) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(output)
        .map_err(|err| format!("failed to serialize output: {}", err))?;
    println!("{}", serialized);
    Ok(())
}

fn append_metrics_log(log_file: &Path, command: &str, payload: &Value) -> Result<(), String> {
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create log dir {}: {}", parent.display(), err))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .map_err(|err| format!("failed to open log file {}: {}", log_file.display(), err))?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("clock error: {}", err))?
        .as_secs();

    let event = serde_json::json!({
        "timestamp_unix": ts,
        "command": command,
        "payload": payload,
    });

    let line = serde_json::to_string(&event)
        .map_err(|err| format!("failed to serialize log line: {}", err))?;
    writeln!(file, "{}", line)
        .map_err(|err| format!("failed writing log file {}: {}", log_file.display(), err))
}

fn current_process_memory_bytes() -> Option<u64> {
    let mut system = System::new_all();
    system.refresh_processes();
    let pid = Pid::from_u32(std::process::id());
    system.process(pid).map(|p| p.memory())
}
