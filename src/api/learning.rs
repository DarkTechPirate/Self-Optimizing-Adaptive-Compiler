use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramFeatures {
    pub has_loop: bool,
    pub num_loops: usize,
    pub num_operations: usize,
    pub num_variables: usize,
    pub repeated_expressions: usize,
    pub branching_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyScore {
    pub strategy: String,
    pub avg_speedup: f64,
    pub consistency: f64,
    pub sample_count: usize,
    pub history_score: f64,
    pub llm_confidence: f64,
    #[serde(default)]
    pub speed_score: f64,
    #[serde(default)]
    pub memory_score: f64,
    #[serde(default)]
    pub cost_score: f64,
    #[serde(default)]
    pub cache_boost: f64,
    #[serde(default)]
    pub reason: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDecision {
    pub candidate_strategies: Vec<String>,
    pub selected_strategies: Vec<String>,
    pub strategy_scores: Vec<StrategyScore>,
    pub reused_history: bool,
    #[serde(default)]
    pub program_cached_strategies: Vec<String>,
    #[serde(default)]
    pub program_cache_hit: bool,
    #[serde(default)]
    pub retained_history_events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvent {
    pub timestamp_unix: u64,
    pub program_hash: String,
    pub input_features: ProgramFeatures,
    pub llm_suggestions: Vec<String>,
    pub candidate_strategies: Vec<String>,
    pub selected_strategies: Vec<String>,
    pub strategy_scores: Vec<StrategyScore>,
    pub applied_passes: Vec<String>,
    pub speedup: f64,
    pub mode: String,
    #[serde(default)]
    pub execution_time_before_us: u64,
    #[serde(default)]
    pub execution_time_after_us: u64,
    #[serde(default)]
    pub time_saved_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySummary {
    pub strategy: String,
    pub runs: usize,
    pub avg_speedup: f64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSavedMetrics {
    pub total_time_saved_us: u64,
    pub time_saved_today_us: u64,
}

pub fn learning_log_path(metrics_log: &Path) -> PathBuf {
    metrics_log
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("learning.jsonl")
}

pub fn program_hash(source: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn extract_program_features(source: &str) -> ProgramFeatures {
    let lower = source.to_lowercase();

    let num_for = lower.matches("for ").count();
    let num_while = lower.matches("while ").count();
    let num_loops = num_for + num_while;

    let mut variables = HashSet::new();
    let mut rhs_counts: HashMap<String, usize> = HashMap::new();
    let mut branch_depth = 0usize;
    let mut max_branch_depth = 0usize;

    for line in source.lines() {
        let trimmed = line.trim();

        let mut rest = trimmed;
        while let Some(idx) = rest.find("let ") {
            // Ignore matches embedded in identifiers.
            if idx > 0 {
                let prev = rest.as_bytes()[idx - 1] as char;
                if prev.is_alphanumeric() || prev == '_' {
                    rest = &rest[idx + 4..];
                    continue;
                }
            }

            let after_let = &rest[idx + 4..];
            let name = after_let
                .split(|c: char| c == '=' || c.is_whitespace() || c == ';' || c == '{' || c == '}')
                .next()
                .unwrap_or("")
                .trim();
            if !name.is_empty() {
                variables.insert(name.to_string());
            }
            rest = after_let;
        }

        if let Some(idx) = trimmed.find('=') {
            let rhs = trimmed[idx + 1..].trim();
            if !rhs.is_empty() {
                *rhs_counts.entry(rhs.to_string()).or_insert(0) += 1;
            }
        }

        let if_count = trimmed.matches("if ").count();
        if if_count > 0 {
            branch_depth += if_count;
            max_branch_depth = max_branch_depth.max(branch_depth);
        }

        let close_count = trimmed.matches('}').count();
        if close_count > 0 {
            branch_depth = branch_depth.saturating_sub(close_count);
        }
    }

    let repeated_expressions = rhs_counts
        .values()
        .filter(|&&count| count > 1)
        .map(|count| count - 1)
        .sum();

    ProgramFeatures {
        has_loop: num_loops > 0,
        num_loops,
        num_operations: count_operations(source),
        num_variables: variables.len(),
        repeated_expressions,
        branching_depth: max_branch_depth,
    }
}

pub fn normalize_strategy(strategy: &str) -> String {
    let lower = strategy.trim().to_lowercase();
    let compact = lower.replace('-', "_").replace(' ', "_");

    if compact.contains("constant") {
        "constant_propagation".to_string()
    } else if compact.contains("dead_code") || compact.contains("dead") {
        "dead_code_elimination".to_string()
    } else if compact.contains("loop_invariant") {
        "loop_invariant_motion".to_string()
    } else if compact.contains("strength") {
        "strength_reduction".to_string()
    } else if compact.contains("inline") {
        "inline_function".to_string()
    } else if compact.contains("unroll") {
        "loop_unrolling".to_string()
    } else if compact.contains("cse") || compact.contains("common_subexpression") {
        "common_subexpression_elimination".to_string()
    } else if compact.contains("peephole") {
        "peephole".to_string()
    } else if compact.contains("vector") || compact.contains("simd") {
        "vectorize".to_string()
    } else {
        compact
    }
}

pub fn mode_default_strategies(mode: &str) -> Vec<String> {
    match mode {
        "speed" => vec![
            "constant_propagation".to_string(),
            "dead_code_elimination".to_string(),
            "loop_invariant_motion".to_string(),
            "strength_reduction".to_string(),
            "common_subexpression_elimination".to_string(),
            "peephole".to_string(),
            "loop_unrolling".to_string(),
            "inline_function".to_string(),
            "vectorize".to_string(),
        ],
        "memory" => vec![
            "dead_code_elimination".to_string(),
            "constant_propagation".to_string(),
            "peephole".to_string(),
            "common_subexpression_elimination".to_string(),
        ],
        "auto" => vec![
            "constant_propagation".to_string(),
            "dead_code_elimination".to_string(),
            "loop_invariant_motion".to_string(),
            "strength_reduction".to_string(),
            "common_subexpression_elimination".to_string(),
            "peephole".to_string(),
            "loop_unrolling".to_string(),
            "inline_function".to_string(),
            "vectorize".to_string(),
        ],
        _ => vec![
            "constant_propagation".to_string(),
            "dead_code_elimination".to_string(),
            "loop_invariant_motion".to_string(),
            "common_subexpression_elimination".to_string(),
            "peephole".to_string(),
        ],
    }
}

pub fn threshold_for_mode(mode: &str) -> f64 {
    match mode {
        "speed" => 0.55,
        "memory" => 0.50,
        "balanced" => 0.48,
        _ => 0.45,
    }
}

pub fn select_strategies(
    mode_defaults: &[String],
    llm_suggestions: &[(String, f32)],
    features: &ProgramFeatures,
    history: &[LearningEvent],
    threshold: f64,
    mode: &str,
    program_hash: &str,
) -> StrategyDecision {
    let retained_history = retain_history(history);
    let program_cached = build_program_cache(&retained_history, program_hash, 3);
    let cache_boosts = cache_boosts(&program_cached);

    let normalized_defaults: Vec<String> = mode_defaults
        .iter()
        .map(|s| normalize_strategy(s))
        .collect();

    let mut llm_confidence: HashMap<String, f64> = HashMap::new();
    for (strategy, confidence) in llm_suggestions {
        let key = normalize_strategy(strategy);
        let entry = llm_confidence.entry(key).or_insert(0.0);
        *entry = (*entry).max(*confidence as f64);
    }

    let mut candidates: HashSet<String> = normalized_defaults.iter().cloned().collect();
    candidates.extend(llm_confidence.keys().cloned());
    candidates.extend(program_cached.iter().cloned());

    let mut strategy_scores = Vec::new();
    for candidate in candidates {
        let llm = *llm_confidence.get(&candidate).unwrap_or(&0.0);
        let is_default = normalized_defaults.contains(&candidate);
        let cache_boost = *cache_boosts.get(&candidate).unwrap_or(&0.0);
        strategy_scores.push(score_strategy(
            &candidate,
            llm,
            is_default,
            features,
            &retained_history,
            mode,
            cache_boost,
        ));
    }

    strategy_scores.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut selected_strategies: Vec<String> = strategy_scores
        .iter()
        .filter(|score| score.score >= threshold)
        .map(|score| score.strategy.clone())
        .collect();

    if selected_strategies.is_empty() {
        selected_strategies = normalized_defaults;
    }

    if selected_strategies.is_empty() && !strategy_scores.is_empty() {
        selected_strategies.push(strategy_scores[0].strategy.clone());
    }

    dedupe(&mut selected_strategies);

    let reused_history = selected_strategies.iter().any(|selected| {
        strategy_scores
            .iter()
            .any(|score| score.strategy == *selected && score.sample_count > 0)
    });

    let program_cache_hit = selected_strategies
        .iter()
        .any(|strategy| program_cached.contains(strategy));

    StrategyDecision {
        candidate_strategies: strategy_scores
            .iter()
            .map(|score| score.strategy.clone())
            .collect(),
        selected_strategies,
        strategy_scores,
        reused_history,
        program_cached_strategies: program_cached,
        program_cache_hit,
        retained_history_events: retained_history.len(),
    }
}

pub fn create_learning_event(
    source: &str,
    features: ProgramFeatures,
    llm_suggestions: Vec<String>,
    decision: &StrategyDecision,
    applied_passes: Vec<String>,
    speedup: f64,
    mode: String,
    execution_time_before_us: u64,
    execution_time_after_us: u64,
) -> LearningEvent {
    LearningEvent {
        timestamp_unix: unix_timestamp_now(),
        program_hash: program_hash(source),
        input_features: features,
        llm_suggestions,
        candidate_strategies: decision.candidate_strategies.clone(),
        selected_strategies: decision.selected_strategies.clone(),
        strategy_scores: decision.strategy_scores.clone(),
        applied_passes,
        speedup,
        mode,
        execution_time_before_us,
        execution_time_after_us,
        time_saved_us: execution_time_before_us.saturating_sub(execution_time_after_us),
    }
}

pub fn time_saved_metrics(events: &[LearningEvent]) -> TimeSavedMetrics {
    let current_day = unix_timestamp_now() / 86_400;

    let total_time_saved_us = events
        .iter()
        .map(|event| event.time_saved_us)
        .sum();

    let time_saved_today_us = events
        .iter()
        .filter(|event| event.timestamp_unix / 86_400 == current_day)
        .map(|event| event.time_saved_us)
        .sum();

    TimeSavedMetrics {
        total_time_saved_us,
        time_saved_today_us,
    }
}

pub fn append_learning_event(path: &Path, event: &LearningEvent) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create learning dir {}: {}", parent.display(), err))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("failed to open learning file {}: {}", path.display(), err))?;

    let line = serde_json::to_string(event)
        .map_err(|err| format!("failed to serialize learning event: {}", err))?;
    writeln!(file, "{}", line)
        .map_err(|err| format!("failed writing learning file {}: {}", path.display(), err))
}

pub fn read_learning_events(path: &Path, limit: usize) -> Vec<LearningEvent> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut events = Vec::new();
    for line in content.lines().rev().take(limit) {
        if let Ok(event) = serde_json::from_str::<LearningEvent>(line) {
            events.push(event);
        }
    }
    events.reverse();
    events
}

pub fn summarize_strategies(events: &[LearningEvent], top_n: usize) -> Vec<StrategySummary> {
    let mut strategy_map: HashMap<String, Vec<f64>> = HashMap::new();

    for event in events {
        let mut seen = HashSet::new();
        for pass in &event.applied_passes {
            let strategy = normalize_applied_pass(pass);
            if seen.insert(strategy.clone()) {
                strategy_map.entry(strategy).or_default().push(event.speedup);
            }
        }
    }

    let mut summary: Vec<StrategySummary> = strategy_map
        .into_iter()
        .map(|(strategy, speedups)| {
            let runs = speedups.len();
            let sum: f64 = speedups.iter().sum();
            let avg_speedup = if runs > 0 { sum / runs as f64 } else { 0.0 };
            let success_count = speedups.iter().filter(|&&s| s > 1.0).count();
            let success_rate = if runs > 0 {
                success_count as f64 / runs as f64
            } else {
                0.0
            };

            StrategySummary {
                strategy,
                runs,
                avg_speedup,
                success_rate,
            }
        })
        .collect();

    summary.sort_by(|a, b| {
        let sa = a.avg_speedup * (0.5 + 0.5 * a.success_rate);
        let sb = b.avg_speedup * (0.5 + 0.5 * b.success_rate);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    summary.truncate(top_n);
    summary
}

fn score_strategy(
    strategy: &str,
    llm_confidence: f64,
    is_mode_default: bool,
    target_features: &ProgramFeatures,
    history: &[LearningEvent],
    mode: &str,
    cache_boost: f64,
) -> StrategyScore {
    let mut matched_speedups = Vec::new();
    let mut weighted_speedup_sum = 0.0;
    let mut weighted_total = 0.0;

    for event in history {
        let applies = event
            .applied_passes
            .iter()
            .any(|pass| normalize_applied_pass(pass) == strategy);
        if !applies {
            continue;
        }

        let similarity = feature_similarity(target_features, &event.input_features);
        let weight = 0.25 + (0.75 * similarity);
        matched_speedups.push(event.speedup);
        weighted_speedup_sum += event.speedup * weight;
        weighted_total += weight;
    }

    let sample_count = matched_speedups.len();
    let (speed_score, memory_score, cost_reason) = cost_model_scores(strategy, target_features, mode);
    let cost_score = combine_cost_scores(speed_score, memory_score, mode);

    if sample_count == 0 {
        let prior = if is_mode_default { 0.42 } else { 0.28 };
        let score = (
            (0.45 * prior)
                + (0.30 * cost_score)
                + (0.20 * llm_confidence)
                + cache_boost
        )
            .clamp(0.0, 1.0);

        let reason = format!(
            "prior={:.2}, llm={:.2}, cost={:.2} ({}), cache={:.2}",
            prior, llm_confidence, cost_score, cost_reason, cache_boost
        );

        return StrategyScore {
            strategy: strategy.to_string(),
            avg_speedup: 1.0,
            consistency: 0.5,
            sample_count,
            history_score: prior,
            llm_confidence,
            speed_score,
            memory_score,
            cost_score,
            cache_boost,
            reason,
            score,
        };
    }

    let avg_speedup = if weighted_total > 0.0 {
        weighted_speedup_sum / weighted_total
    } else {
        1.0
    };

    let mean = matched_speedups.iter().sum::<f64>() / sample_count as f64;
    let variance = matched_speedups
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / sample_count as f64;
    let std_dev = variance.sqrt();

    let consistency = (1.0 / (1.0 + std_dev)).clamp(0.0, 1.0);
    let speed_component = ((avg_speedup - 1.0) / 2.5).clamp(0.0, 1.0);
    let sample_component = (sample_count as f64 / 8.0).min(1.0);

    let history_score =
        (0.55 * speed_component) + (0.25 * consistency) + (0.20 * sample_component);

    let mode_boost = if is_mode_default { 0.10 } else { 0.0 };
    let score = (
        (history_score * 0.55)
            + (llm_confidence * 0.20)
            + (cost_score * 0.20)
            + mode_boost
            + cache_boost
    )
        .clamp(0.0, 1.0);

    let reason = format!(
        "history={:.2}, llm={:.2}, cost={:.2} ({}), cache={:.2}",
        history_score, llm_confidence, cost_score, cost_reason, cache_boost
    );

    StrategyScore {
        strategy: strategy.to_string(),
        avg_speedup,
        consistency,
        sample_count,
        history_score,
        llm_confidence,
        speed_score,
        memory_score,
        cost_score,
        cache_boost,
        reason,
        score,
    }
}

fn cost_model_scores(strategy: &str, features: &ProgramFeatures, mode: &str) -> (f64, f64, String) {
    let has_loop = features.has_loop;
    let op_scale = (features.num_operations as f64 / 50.0).clamp(0.0, 1.0);
    let repeated = (features.repeated_expressions as f64 / 8.0).clamp(0.0, 1.0);

    let (mut speed_score, mut memory_score) = match strategy {
        "constant_propagation" => (0.60, 0.90),
        "dead_code_elimination" => (0.55, 0.95),
        "loop_invariant_motion" => (if has_loop { 0.70 } else { 0.35 }, 0.70),
        "strength_reduction" => (if features.num_operations > 0 { 0.65 } else { 0.40 }, 0.75),
        "common_subexpression_elimination" => (0.55 + 0.25 * repeated, 0.75),
        "peephole" => (0.45 + 0.15 * op_scale, 0.90),
        "loop_unrolling" => (if has_loop { 0.72 } else { 0.40 }, 0.35),
        "inline_function" => (0.55, 0.40),
        "vectorize" => (if has_loop && features.num_operations > 8 { 0.68 } else { 0.30 }, 0.40),
        _ => (0.45, 0.60),
    };

    let size_penalty = match strategy {
        "loop_unrolling" | "inline_function" | "vectorize" => 0.20 * op_scale,
        _ => 0.05 * op_scale,
    };
    memory_score = (memory_score - size_penalty).clamp(0.0, 1.0);

    if mode == "memory" {
        speed_score = (speed_score * 0.90).clamp(0.0, 1.0);
    }

    let reason = format!(
        "speed={:.2}, memory={:.2}",
        speed_score, memory_score
    );

    (speed_score, memory_score, reason)
}

fn combine_cost_scores(speed_score: f64, memory_score: f64, mode: &str) -> f64 {
    let (speed_weight, memory_weight) = match mode {
        "speed" => (0.70, 0.30),
        "memory" => (0.35, 0.65),
        "balanced" => (0.55, 0.45),
        _ => (0.60, 0.40),
    };

    (speed_weight * speed_score + memory_weight * memory_score).clamp(0.0, 1.0)
}

fn retain_history(history: &[LearningEvent]) -> Vec<LearningEvent> {
    const MAX_HISTORY_EVENTS: usize = 1500;
    const MAX_PROGRAM_EVENTS: usize = 40;
    const MAX_HISTORY_AGE_SECS: u64 = 60 * 60 * 24 * 30;

    let now = unix_timestamp_now();
    let min_timestamp = now.saturating_sub(MAX_HISTORY_AGE_SECS);

    let mut per_program_counts: HashMap<String, usize> = HashMap::new();
    let mut retained_rev = Vec::new();

    for event in history.iter().rev() {
        if retained_rev.len() >= MAX_HISTORY_EVENTS {
            break;
        }

        if event.timestamp_unix < min_timestamp {
            continue;
        }

        let count = per_program_counts.entry(event.program_hash.clone()).or_insert(0);
        if *count >= MAX_PROGRAM_EVENTS {
            continue;
        }

        *count += 1;
        retained_rev.push(event.clone());
    }

    retained_rev.reverse();
    retained_rev
}

fn build_program_cache(
    history: &[LearningEvent],
    program_hash: &str,
    top_n: usize,
) -> Vec<String> {
    let mut score_map: HashMap<String, (f64, usize)> = HashMap::new();

    for event in history {
        if event.program_hash != program_hash {
            continue;
        }

        let mut seen = HashSet::new();
        for pass in &event.applied_passes {
            let strategy = normalize_applied_pass(pass);
            if !seen.insert(strategy.clone()) {
                continue;
            }
            let entry = score_map.entry(strategy).or_insert((0.0, 0));
            entry.0 += event.speedup;
            entry.1 += 1;
        }
    }

    let mut ranked: Vec<(String, f64)> = score_map
        .into_iter()
        .map(|(strategy, (sum, count))| (strategy, if count > 0 { sum / count as f64 } else { 0.0 }))
        .collect();

    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.into_iter().take(top_n).map(|(s, _)| s).collect()
}

fn cache_boosts(cached: &[String]) -> HashMap<String, f64> {
    let mut boosts = HashMap::new();
    for (idx, strategy) in cached.iter().enumerate() {
        let boost = match idx {
            0 => 0.12,
            1 => 0.08,
            2 => 0.05,
            _ => 0.02,
        };
        boosts.insert(strategy.clone(), boost);
    }
    boosts
}

fn feature_similarity(a: &ProgramFeatures, b: &ProgramFeatures) -> f64 {
    let mut total = 0.0;

    total += bool_similarity(a.has_loop, b.has_loop);
    total += numeric_similarity(a.num_loops, b.num_loops);
    total += numeric_similarity(a.num_operations, b.num_operations);
    total += numeric_similarity(a.num_variables, b.num_variables);
    total += numeric_similarity(a.repeated_expressions, b.repeated_expressions);
    total += numeric_similarity(a.branching_depth, b.branching_depth);

    (total / 6.0).clamp(0.0, 1.0)
}

fn bool_similarity(a: bool, b: bool) -> f64 {
    if a == b { 1.0 } else { 0.0 }
}

fn numeric_similarity(a: usize, b: usize) -> f64 {
    if a == 0 && b == 0 {
        return 1.0;
    }

    let max = a.max(b) as f64;
    let diff = a.abs_diff(b) as f64;
    (1.0 - (diff / max)).clamp(0.0, 1.0)
}

fn count_operations(source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut i = 0usize;
    let mut count = 0usize;

    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            '+' | '-' | '*' | '/' | '%' => {
                count += 1;
                i += 1;
            }
            '<' | '>' | '!' | '=' => {
                if i + 1 < bytes.len() && bytes[i + 1] as char == '=' {
                    count += 1;
                    i += 2;
                } else if c == '<' || c == '>' {
                    count += 1;
                    i += 1;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    count
}

fn normalize_applied_pass(pass: &str) -> String {
    let lower = pass.trim().to_lowercase();
    match lower.as_str() {
        "constant_folding" => "constant_propagation".to_string(),
        "loop_invariant_code_motion" => "loop_invariant_motion".to_string(),
        _ => normalize_strategy(&lower),
    }
}

fn dedupe(strategies: &mut Vec<String>) {
    let mut seen = HashSet::new();
    strategies.retain(|s| seen.insert(s.clone()));
}

fn unix_timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
