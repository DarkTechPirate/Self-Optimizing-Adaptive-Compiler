use super::input::normalize_source_input;
use super::learning::{
    append_learning_event, create_learning_event, extract_program_features, learning_log_path,
    mode_default_strategies, program_hash, read_learning_events, select_strategies,
    summarize_strategies, threshold_for_mode, time_saved_metrics,
};
use super::{NyxCompiler, ProfileResult};
use crate::llm::{LLMClient, OptimizationSuggestion};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, System};

#[derive(Clone)]
struct ApiState {
    log_file: PathBuf,
    learning_file: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    pub source: String,
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub source: String,
    #[serde(default)]
    pub no_llm: bool,
}

#[derive(Debug, Deserialize)]
pub struct OptimizeRequest {
    pub source: String,
    pub mode: Option<String>,
    #[serde(default)]
    pub no_llm: bool,
}

#[derive(Debug, Deserialize)]
pub struct RecentMetricsQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct LearningSummaryQuery {
    pub limit: Option<usize>,
}

type ApiResult = Result<Json<Value>, (StatusCode, Json<Value>)>;

pub async fn run_server(host: String, port: u16, log_file: PathBuf) -> Result<(), String> {
    let learning_file = learning_log_path(&log_file);
    let state = ApiState {
        log_file,
        learning_file,
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/execute", post(execute_handler))
        .route("/analyze", post(analyze_handler))
        .route("/optimize", post(optimize_handler))
        .route("/metrics/recent", get(metrics_recent_handler))
        .route("/learning/summary", get(learning_summary_handler))
        .route("/dashboard", get(dashboard_handler))
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    eprintln!("Nyx API listening on http://{}", addr);
    eprintln!("Dashboard: http://{}/dashboard", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|err| format!("failed to bind {}: {}", addr, err))?;

    axum::serve(listener, app)
        .await
        .map_err(|err| format!("server error: {}", err))
}

async fn health_handler() -> Json<Value> {
    Json(json!({
        "success": true,
        "service": "nyx-runtime-api",
        "status": "ok",
    }))
}

async fn execute_handler(
    State(state): State<ApiState>,
    Json(request): Json<ExecuteRequest>,
) -> ApiResult {
    if request.source.trim().is_empty() {
        return Err(api_error(StatusCode::BAD_REQUEST, "source must not be empty"));
    }

    let normalized = normalize_source_input(&request.source);

    let mut compiler = NyxCompiler::new();
    let compile = compiler.compile(&normalized.source);
    if !compile.success {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            &compile
                .error
                .unwrap_or_else(|| "failed to compile input".to_string()),
        ));
    }

    let execute = compiler.execute();
    let features = extract_program_features(&normalized.source);
    let output = json!({
        "success": true,
        "endpoint": "execute",
        "input_format": normalized.input_format,
        "source_normalized": normalized.normalization_applied,
        "program_hash": program_hash(&normalized.source),
        "input_features": features,
        "execution_time_us": execute.total_time_us,
        "total_instructions": execute.total_instructions,
        "hot_instruction_count": execute.hot_instruction_count,
        "process_memory_bytes": current_process_memory_bytes(),
        "return_value": execute.return_value,
    });

    if let Err(err) = append_metrics_log(&state.log_file, "api_execute", &output) {
        eprintln!("metrics logging failed: {}", err);
    }

    Ok(Json(output))
}

async fn analyze_handler(
    State(state): State<ApiState>,
    Json(request): Json<AnalyzeRequest>,
) -> ApiResult {
    if request.source.trim().is_empty() {
        return Err(api_error(StatusCode::BAD_REQUEST, "source must not be empty"));
    }

    let normalized = normalize_source_input(&request.source);

    let mut compiler = NyxCompiler::new();
    let compile = compiler.compile(&normalized.source);
    if !compile.success {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            &compile
                .error
                .unwrap_or_else(|| "failed to compile input".to_string()),
        ));
    }

    let execute = compiler.execute();
    let profile = compiler.profile();
    let analysis = compiler.analyze();
    let features = extract_program_features(&normalized.source);
    let history = read_learning_events(&state.learning_file, 1000);
    let strategy_success = summarize_strategies(&history, 12);
    let savings = time_saved_metrics(&history);
    let (llm_status, llm_suggestions) =
        maybe_collect_llm_suggestions(&profile, request.no_llm).await;

    let output = json!({
        "success": true,
        "endpoint": "analyze",
        "input_format": normalized.input_format,
        "source_normalized": normalized.normalization_applied,
        "program_hash": program_hash(&normalized.source),
        "input_features": features,
        "llm_status": llm_status,
        "execution_time_us": execute.total_time_us,
        "total_time_saved_us": savings.total_time_saved_us,
        "time_saved_today_us": savings.time_saved_today_us,
        "hot_instruction_count": execute.hot_instruction_count,
        "process_memory_bytes": current_process_memory_bytes(),
        "profile": profile,
        "analysis": analysis,
        "strategy_success_rates": strategy_success,
        "llm_suggestions": llm_suggestions,
    });

    if let Err(err) = append_metrics_log(&state.log_file, "api_analyze", &output) {
        eprintln!("metrics logging failed: {}", err);
    }

    Ok(Json(output))
}

async fn optimize_handler(
    State(state): State<ApiState>,
    Json(request): Json<OptimizeRequest>,
) -> ApiResult {
    if request.source.trim().is_empty() {
        return Err(api_error(StatusCode::BAD_REQUEST, "source must not be empty"));
    }

    let normalized = normalize_source_input(&request.source);

    let mut compiler = NyxCompiler::new();
    let compile = compiler.compile(&normalized.source);
    if !compile.success {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            &compile
                .error
                .unwrap_or_else(|| "failed to compile input".to_string()),
        ));
    }

    let baseline_exec = compiler.execute();
    let baseline_profile = compiler.profile();
    let mode = normalize_mode(request.mode.as_deref());
    let features = extract_program_features(&normalized.source);
    let hash = program_hash(&normalized.source);
    let history = read_learning_events(&state.learning_file, 1000);
    let (llm_status, llm_suggestions) =
        maybe_collect_llm_suggestions(&baseline_profile, request.no_llm).await;

    let mode_defaults = mode_default_strategies(mode);
    let llm_inputs: Vec<(String, f32)> = llm_suggestions
        .iter()
        .map(|s| (s.strategy.clone(), s.confidence))
        .collect();

    let decision = select_strategies(
        &mode_defaults,
        &llm_inputs,
        &features,
        &history,
        threshold_for_mode(mode),
        mode,
        &hash,
    );

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
        mode.to_string(),
        baseline_exec.total_time_us,
        optimized_exec.total_time_us,
    );
    if let Err(err) = append_learning_event(&state.learning_file, &learning_event) {
        eprintln!("learning logging failed: {}", err);
    }

    let mut history_with_current = history.clone();
    history_with_current.push(learning_event.clone());
    let savings = time_saved_metrics(&history_with_current);

    let output = json!({
        "success": true,
        "endpoint": "optimize",
        "mode": mode,
        "input_format": normalized.input_format,
        "source_normalized": normalized.normalization_applied,
        "program_hash": learning_event.program_hash,
        "input_features": features,
        "llm_status": llm_status,
        "execution_time_before_us": baseline_exec.total_time_us,
        "execution_time_after_us": optimized_exec.total_time_us,
        "time_saved_us": learning_event.time_saved_us,
        "total_time_saved_us": savings.total_time_saved_us,
        "time_saved_today_us": savings.time_saved_today_us,
        "speedup_ratio": speedup_ratio,
        "historical_event_count": history.len(),
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
        "process_memory_bytes": current_process_memory_bytes(),
        "baseline_return_value": baseline_return_value,
        "optimized_return_value": optimized_return_value,
        "correctness_verified": correctness_verified,
        "return_value": optimized_exec.return_value,
    });

    if let Err(err) = append_metrics_log(&state.log_file, "api_optimize", &output) {
        eprintln!("metrics logging failed: {}", err);
    }

    Ok(Json(output))
}

async fn metrics_recent_handler(
    State(state): State<ApiState>,
    Query(query): Query<RecentMetricsQuery>,
) -> Json<Value> {
    let limit = query.limit.unwrap_or(20).min(200);
    let events = read_recent_metrics(&state.log_file, limit);

    Json(json!({
        "success": true,
        "count": events.len(),
        "events": events,
    }))
}

async fn learning_summary_handler(
    State(state): State<ApiState>,
    Query(query): Query<LearningSummaryQuery>,
) -> Json<Value> {
    let limit = query.limit.unwrap_or(20).min(200);
    let events = read_learning_events(&state.learning_file, 5000);
    let summary = summarize_strategies(&events, limit);
    let savings = time_saved_metrics(&events);

    Json(json!({
        "success": true,
        "learning_event_count": events.len(),
        "total_time_saved_us": savings.total_time_saved_us,
        "time_saved_today_us": savings.time_saved_today_us,
        "strategies": summary,
    }))
}

async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn maybe_collect_llm_suggestions(
    profile: &ProfileResult,
    no_llm: bool,
) -> (String, Vec<OptimizationSuggestion>) {
    if no_llm {
        return ("disabled".to_string(), Vec::new());
    }

    let profile_json = serde_json::to_string_pretty(profile).unwrap_or_else(|_| "{}".to_string());

    tokio::task::spawn_blocking(move || {
        let client = LLMClient::new();
        if !client.is_available() {
            return ("unavailable".to_string(), Vec::new());
        }

        match client.analyze_profile(&profile_json) {
            Ok(analysis) => ("connected".to_string(), analysis.suggestions),
            Err(err) => (format!("analysis_failed: {}", err), Vec::new()),
        }
    })
    .await
    .unwrap_or_else(|err| (format!("analysis_failed: {}", err), Vec::new()))
}

fn normalize_mode(mode: Option<&str>) -> &'static str {
    match mode.unwrap_or("auto").to_lowercase().as_str() {
        "auto" => "auto",
        "speed" => "speed",
        "memory" => "memory",
        _ => "balanced",
    }
}

fn current_process_memory_bytes() -> Option<u64> {
    let mut system = System::new_all();
    system.refresh_processes();
    let pid = Pid::from_u32(std::process::id());
    system.process(pid).map(|p| p.memory())
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

    let event = json!({
        "timestamp_unix": ts,
        "command": command,
        "payload": payload,
    });

    let line = serde_json::to_string(&event)
        .map_err(|err| format!("failed to serialize log line: {}", err))?;
    writeln!(file, "{}", line)
        .map_err(|err| format!("failed writing log file {}: {}", log_file.display(), err))
}

fn read_recent_metrics(path: &Path, limit: usize) -> Vec<Value> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut events = Vec::new();
    for line in content.lines().rev().take(limit) {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            events.push(value);
        }
    }
    events.reverse();
    events
}

fn api_error(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "success": false,
            "error": message,
        })),
    )
}

const DASHBOARD_HTML: &str = r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Nyx Runtime Intelligence Dashboard</title>
  <style>
    :root {
            --bg: #0a0f1e;
            --card: #111a33;
            --text: #d8e6ff;
            --muted: #8ba0c8;
            --accent: #38bdf8;
            --accent-2: #22c55e;
            --danger: #fb923c;
            --border: #1d2b53;
    }
    body {
      margin: 0;
            font-family: "Space Grotesk", "Segoe UI", Helvetica, Arial, sans-serif;
            background:
                radial-gradient(circle at 20% 10%, #1e3a8a55 0%, transparent 40%),
                radial-gradient(circle at 80% 0%, #0ea5e955 0%, transparent 42%),
                var(--bg);
      color: var(--text);
      min-height: 100vh;
    }
    .wrap {
      max-width: 980px;
      margin: 0 auto;
      padding: 24px;
    }
    .title {
            font-size: 30px;
      font-weight: 700;
      margin: 0 0 4px;
    }
    .sub {
      color: var(--muted);
      margin-bottom: 20px;
    }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 12px;
      margin-bottom: 20px;
    }
    .card {
      background: color-mix(in hsl, var(--card) 92%, black);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 14px;
    }
    .k {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }
    .v {
      font-size: 24px;
      margin-top: 8px;
      color: var(--accent);
      font-weight: 700;
    }
        .panel {
            background: color-mix(in hsl, var(--card) 92%, black);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 14px;
            margin-bottom: 16px;
        }
        .panel h2 {
            margin: 0 0 10px;
            font-size: 16px;
        }
        .chart {
            width: 100%;
            height: 220px;
            border: 1px solid var(--border);
            border-radius: 10px;
            background: #0a1326;
        }
        table {
      width: 100%;
      border-collapse: collapse;
      border-radius: 12px;
      overflow: hidden;
      background: color-mix(in hsl, var(--card) 92%, black);
      border: 1px solid var(--border);
    }
    th, td {
      text-align: left;
      padding: 10px 12px;
      border-bottom: 1px solid var(--border);
      font-size: 14px;
    }
    th {
      color: var(--muted);
      font-weight: 600;
    }
        .reason {
            font-size: 13px;
            color: var(--muted);
            margin: 6px 0;
            padding-bottom: 6px;
            border-bottom: 1px dashed var(--border);
        }
    button {
      background: var(--accent);
      border: 0;
      color: #001019;
      font-weight: 700;
      border-radius: 10px;
      padding: 10px 14px;
      cursor: pointer;
      margin-bottom: 14px;
    }
  </style>
</head>
<body>
  <div class="wrap">
        <h1 class="title">Nyx Runtime Intelligence Dashboard</h1>
        <div class="sub">Performance timeline, strategy learning, and LLM reasoning</div>
    <button id="refresh">Refresh Metrics</button>

    <div class="grid">
      <div class="card"><div class="k">Events</div><div class="v" id="events">0</div></div>
      <div class="card"><div class="k">Average Speedup</div><div class="v" id="speedup">-</div></div>
            <div class="card"><div class="k">Learning Events</div><div class="v" id="learning">-</div></div>
            <div class="card"><div class="k">Total Time Saved</div><div class="v" id="savedTotal">-</div></div>
            <div class="card"><div class="k">Nyx Saved You Today</div><div class="v" id="savedToday">-</div></div>
    </div>

        <div class="panel">
            <h2>Before vs After Execution (recent runs)</h2>
            <canvas id="latencyChart" class="chart" width="920" height="220"></canvas>
        </div>

        <div class="panel">
            <h2>Recent LLM Reasoning</h2>
            <div id="reasons"></div>
        </div>

        <div class="panel">
            <h2>Decision Rationale</h2>
            <div id="decisionReasons"></div>
        </div>

        <div class="panel">
            <h2>Per-Program Time Saved</h2>
            <table>
                <thead>
                    <tr>
                        <th>Program</th>
                        <th>Runs</th>
                        <th>Total Saved</th>
                        <th>Avg Speedup</th>
                    </tr>
                </thead>
                <tbody id="programRows"></tbody>
            </table>
        </div>

        <div class="panel">
            <h2>Optimization Timeline</h2>
            <table>
                <thead>
                    <tr>
                        <th>Timestamp</th>
                        <th>Mode</th>
                        <th>Before (us)</th>
                        <th>After (us)</th>
                        <th>Speedup</th>
                        <th>History Reuse</th>
                    </tr>
                </thead>
                <tbody id="timelineRows"></tbody>
            </table>
        </div>

        <div class="panel">
            <h2>Strategy Success Rates</h2>
            <table>
                <thead>
                    <tr>
                        <th>Strategy</th>
                        <th>Runs</th>
                        <th>Avg Speedup</th>
                        <th>Success Rate</th>
                    </tr>
                </thead>
                <tbody id="strategyRows"></tbody>
            </table>
        </div>

        <div class="panel">
            <h2>Strategy Win Rates by Code Shape</h2>
            <table>
                <thead>
                    <tr>
                        <th>Shape</th>
                        <th>Strategy</th>
                        <th>Runs</th>
                        <th>Avg Speedup</th>
                        <th>Success Rate</th>
                    </tr>
                </thead>
                <tbody id="shapeRows"></tbody>
            </table>
        </div>

    <table>
      <thead>
        <tr>
          <th>Timestamp</th>
          <th>Command</th>
          <th>Mode</th>
          <th>Speedup</th>
          <th>LLM</th>
        </tr>
      </thead>
      <tbody id="rows"></tbody>
    </table>
  </div>

  <script>
        function drawLatencyChart(events) {
            const canvas = document.getElementById('latencyChart');
            const ctx = canvas.getContext('2d');
            ctx.clearRect(0, 0, canvas.width, canvas.height);

            if (!events.length) return;

            const max = Math.max(...events.map(e => Math.max(e.before || 0, e.after || 0)), 1);
            const margin = 24;
            const width = canvas.width - margin * 2;
            const height = canvas.height - margin * 2;
            const groupWidth = width / events.length;

            events.forEach((evt, i) => {
                const x0 = margin + i * groupWidth;
                const beforeH = (evt.before / max) * (height - 8);
                const afterH = (evt.after / max) * (height - 8);

                ctx.fillStyle = '#fb923c';
                ctx.fillRect(x0 + 6, margin + height - beforeH, 10, beforeH);

                ctx.fillStyle = '#22c55e';
                ctx.fillRect(x0 + 20, margin + height - afterH, 10, afterH);
            });
        }

        function formatDurationUs(micros) {
            if (!micros || micros <= 0) return '0us';

            if (micros >= 1_000_000) {
                return (micros / 1_000_000).toFixed(2) + 's';
            }
            if (micros >= 1_000) {
                return (micros / 1_000).toFixed(2) + 'ms';
            }
            return Math.round(micros) + 'us';
        }

    async function load() {
            const [metricsRes, learningRes] = await Promise.all([
                fetch('/metrics/recent?limit=200'),
                fetch('/learning/summary?limit=15')
            ]);

            const metricsData = await metricsRes.json();
            const learningData = await learningRes.json();

            const events = metricsData.events || [];
            const strategySummary = learningData.strategies || [];

      document.getElementById('events').textContent = events.length;
            document.getElementById('learning').textContent = learningData.learning_event_count || 0;
        document.getElementById('savedTotal').textContent = formatDurationUs(learningData.total_time_saved_us || 0);
        document.getElementById('savedToday').textContent = formatDurationUs(learningData.time_saved_today_us || 0);

            const optimizeEvents = events.filter(e => e.payload && (e.payload.endpoint === 'optimize' || e.payload.command === 'run' || e.payload.command === 'optimize'));

            const speedups = optimizeEvents
        .map(e => e.payload.speedup_ratio)
        .filter(v => typeof v === 'number');
      const avg = speedups.length ? speedups.reduce((a,b) => a+b, 0) / speedups.length : null;
      document.getElementById('speedup').textContent = avg ? avg.toFixed(2) + 'x' : '-';

            const chartData = optimizeEvents.slice(-12).map(e => ({
                before: e.payload.execution_time_before_us || e.payload.execution_time_us || 0,
                after: e.payload.execution_time_after_us || e.payload.execution_time_us || 0
            }));
            drawLatencyChart(chartData);

            const reasons = [];
            for (const evt of optimizeEvents.slice().reverse()) {
                const suggestions = evt.payload.llm_suggestions || [];
                for (const s of suggestions) {
                    if (s.reason) reasons.push(`${s.strategy || 'strategy'}: ${s.reason}`);
                }
            }
            const reasonBox = document.getElementById('reasons');
            reasonBox.innerHTML = reasons.length
                ? reasons.slice(0, 8).map(r => `<div class='reason'>${r}</div>`).join('')
                : "<div class='reason'>No LLM reasoning captured yet.</div>";

            const decisionReasons = [];
            for (const evt of optimizeEvents.slice().reverse()) {
                const scores = evt.payload.strategy_scores || [];
                const selected = new Set(evt.payload.selected_strategies || []);
                for (const s of scores) {
                    if (selected.size && !selected.has(s.strategy)) continue;
                    if (s.reason) {
                        decisionReasons.push(`${s.strategy}: ${s.reason}`);
                    }
                }
            }
            const decisionBox = document.getElementById('decisionReasons');
            decisionBox.innerHTML = decisionReasons.length
                ? decisionReasons.slice(0, 8).map(r => `<div class='reason'>${r}</div>`).join('')
                : "<div class='reason'>No decision rationale captured yet.</div>";

            const programMap = new Map();
            for (const evt of optimizeEvents) {
                const payload = evt.payload || {};
                const hash = payload.program_hash || 'unknown';
                const entry = programMap.get(hash) || { runs: 0, saved: 0, speedupSum: 0 };
                entry.runs += 1;
                entry.saved += payload.time_saved_us || 0;
                entry.speedupSum += payload.speedup_ratio || 0;
                programMap.set(hash, entry);
            }

            const programRows = document.getElementById('programRows');
            programRows.innerHTML = '';
            const programList = Array.from(programMap.entries())
                .map(([hash, entry]) => ({
                    hash,
                    runs: entry.runs,
                    saved: entry.saved,
                    avgSpeedup: entry.runs ? entry.speedupSum / entry.runs : 0
                }))
                .sort((a, b) => b.saved - a.saved)
                .slice(0, 8);

            for (const item of programList) {
                const tr = document.createElement('tr');
                tr.innerHTML = `<td>${item.hash}</td><td>${item.runs}</td><td>${formatDurationUs(item.saved)}</td><td>${item.avgSpeedup.toFixed(2)}x</td>`;
                programRows.appendChild(tr);
            }

            function shapeFor(features) {
                if (!features) return 'unknown';
                const loops = features.num_loops || 0;
                const branches = features.branching_depth || 0;
                const ops = features.num_operations || 0;
                let shape = 'linear';
                if (loops > 0 && branches > 0) shape = 'loop+branch';
                else if (loops > 0) shape = 'loop-heavy';
                else if (branches > 0) shape = 'branching';
                if (ops > 20) shape += ' compute';
                return shape;
            }

            const shapeMap = new Map();
            for (const evt of optimizeEvents) {
                const payload = evt.payload || {};
                const shape = shapeFor(payload.input_features);
                const strategies = payload.selected_strategies || payload.optimization_decisions || [];
                for (const strategy of strategies) {
                    const key = `${shape}::${strategy}`;
                    const entry = shapeMap.get(key) || { shape, strategy, runs: 0, success: 0, speedupSum: 0 };
                    entry.runs += 1;
                    const speedup = payload.speedup_ratio || 0;
                    entry.speedupSum += speedup;
                    if (speedup > 1.0) entry.success += 1;
                    shapeMap.set(key, entry);
                }
            }

            const shapeRows = document.getElementById('shapeRows');
            shapeRows.innerHTML = '';
            const shapeList = Array.from(shapeMap.values())
                .map(entry => ({
                    ...entry,
                    avgSpeedup: entry.runs ? entry.speedupSum / entry.runs : 0,
                    successRate: entry.runs ? entry.success / entry.runs : 0
                }))
                .sort((a, b) => b.avgSpeedup - a.avgSpeedup)
                .slice(0, 10);

            for (const entry of shapeList) {
                const tr = document.createElement('tr');
                tr.innerHTML = `<td>${entry.shape}</td><td>${entry.strategy}</td><td>${entry.runs}</td><td>${entry.avgSpeedup.toFixed(2)}x</td><td>${Math.round(entry.successRate * 100)}%</td>`;
                shapeRows.appendChild(tr);
            }

            const timelineRows = document.getElementById('timelineRows');
            timelineRows.innerHTML = '';
            for (const evt of optimizeEvents.slice().reverse()) {
        const payload = evt.payload || {};
        const tr = document.createElement('tr');
        const ts = evt.timestamp_unix ? new Date(evt.timestamp_unix * 1000).toLocaleString() : '-';
                const before = payload.execution_time_before_us || payload.execution_time_us || '-';
                const after = payload.execution_time_after_us || payload.execution_time_us || '-';
        const speedup = typeof payload.speedup_ratio === 'number' ? payload.speedup_ratio.toFixed(2) + 'x' : '-';
                tr.innerHTML = `<td>${ts}</td><td>${payload.mode || '-'}</td><td>${before}</td><td>${after}</td><td>${speedup}</td><td>${payload.reused_history ? 'yes' : 'no'}</td>`;
                timelineRows.appendChild(tr);
      }

            const strategyRows = document.getElementById('strategyRows');
            strategyRows.innerHTML = '';
            for (const s of strategySummary) {
                const tr = document.createElement('tr');
                tr.innerHTML = `<td>${s.strategy}</td><td>${s.runs}</td><td>${Number(s.avg_speedup || 0).toFixed(2)}x</td><td>${Math.round((s.success_rate || 0) * 100)}%</td>`;
                strategyRows.appendChild(tr);
            }

            const rows = document.getElementById('rows');
            rows.innerHTML = '';
            for (const evt of events.slice().reverse()) {
                const payload = evt.payload || {};
                const tr = document.createElement('tr');
                const ts = evt.timestamp_unix ? new Date(evt.timestamp_unix * 1000).toLocaleString() : '-';
                const cmd = payload.command || payload.endpoint || evt.command || '-';
                const mode = payload.mode || '-';
                const speedup = typeof payload.speedup_ratio === 'number' ? payload.speedup_ratio.toFixed(2) + 'x' : '-';
                const llm = payload.llm_status || '-';
                tr.innerHTML = `<td>${ts}</td><td>${cmd}</td><td>${mode}</td><td>${speedup}</td><td>${llm}</td>`;
                rows.appendChild(tr);
            }
    }

    document.getElementById('refresh').addEventListener('click', load);
    load();
  </script>
</body>
</html>
"#;
