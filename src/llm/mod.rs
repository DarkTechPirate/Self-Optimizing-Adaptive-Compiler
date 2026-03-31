//! LLM Integration Module for Nyx Compiler
//! 
//! Connects to Ollama (TinyLLaMA) for AI-assisted optimization decisions.

pub mod interface;

use serde::{Deserialize, Serialize};
use std::error::Error;

/// Ollama API endpoint
const OLLAMA_URL: &str = "http://127.0.0.1:11434/api/generate";
const MODEL: &str = "tinyllama";

/// LLM client for optimization suggestions
pub struct LLMClient {
    client: reqwest::blocking::Client,
    model: String,
}

/// Request payload for Ollama API
#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// Response from Ollama API
#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

/// Optimization suggestion from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub strategy: String,
    pub reason: String,
    pub confidence: f32,
}

/// Result from LLM analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMAnalysis {
    pub suggestions: Vec<OptimizationSuggestion>,
    pub raw_response: String,
}

impl LLMClient {
    /// Create a new LLM client
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            model: MODEL.to_string(),
        }
    }

    /// Create with custom model
    pub fn with_model(model: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            model: model.to_string(),
        }
    }

    /// Check if Ollama is running
    pub fn is_available(&self) -> bool {
        self.client
            .get("http://127.0.0.1:11434/api/version")
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Send a raw prompt to the LLM
    pub fn query(&self, prompt: &str) -> Result<String, Box<dyn Error>> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self.client
            .post(OLLAMA_URL)
            .json(&request)
            .send()?;

        let ollama_response: OllamaResponse = response.json()?;
        Ok(ollama_response.response)
    }

    /// Analyze compiler profile and suggest optimizations
    pub fn analyze_profile(&self, profile_json: &str) -> Result<LLMAnalysis, Box<dyn Error>> {
        let prompt = format!(
            r#"You are an expert compiler optimization assistant. Analyze this compiler profile data and suggest optimizations.

Profile data:
{}

Based on this profile, suggest optimizations. Respond ONLY with a JSON object in this exact format:
{{
  "suggestions": [
    {{"strategy": "strategy_name", "reason": "why this helps", "confidence": 0.8}}
  ]
}}

Available strategies:
- "unroll_loop" - Unroll small loops
- "inline_function" - Inline frequently called functions  
- "constant_propagation" - Propagate constant values
- "dead_code_elimination" - Remove unused code
- "strength_reduction" - Replace expensive ops with cheaper ones
- "loop_invariant_motion" - Move invariant code out of loops

Respond with JSON only, no explanation:"#,
            profile_json
        );

        let response = self.query(&prompt)?;
        
        // Try to parse as JSON
        let analysis = match serde_json::from_str::<LLMAnalysis>(&response) {
            Ok(parsed) => parsed,
            Err(_) => {
                // If parsing fails, extract suggestions heuristically
                LLMAnalysis {
                    suggestions: self.extract_suggestions(&response),
                    raw_response: response,
                }
            }
        };

        Ok(analysis)
    }

    /// Extract suggestions from free-form text response
    fn extract_suggestions(&self, response: &str) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();
        let lower = response.to_lowercase();

        let strategies = [
            ("unroll_loop", "loop unroll"),
            ("inline_function", "inline"),
            ("constant_propagation", "constant propagation"),
            ("dead_code_elimination", "dead code"),
            ("strength_reduction", "strength reduction"),
            ("loop_invariant_motion", "loop invariant"),
        ];

        for (strategy, keyword) in strategies {
            if lower.contains(keyword) {
                suggestions.push(OptimizationSuggestion {
                    strategy: strategy.to_string(),
                    reason: format!("LLM suggested {} based on profile", strategy),
                    confidence: 0.6,
                });
            }
        }

        // If hot instructions detected, suggest optimizations
        if lower.contains("hot") || lower.contains("frequently") {
            suggestions.push(OptimizationSuggestion {
                strategy: "inline_function".to_string(),
                reason: "Hot code path detected - consider inlining".to_string(),
                confidence: 0.7,
            });
        }

        suggestions
    }

    /// Quick optimization check - returns simple suggestions
    pub fn suggest_quick(&self, code: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let prompt = format!(
            r#"Analyze this code and list 1-3 compiler optimizations that could help. Be brief.

Code:
{}

List optimizations (one per line):"#,
            code
        );

        let response = self.query(&prompt)?;
        let suggestions: Vec<String> = response
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .take(3)
            .collect();

        Ok(suggestions)
    }
}

impl Default for LLMClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to check if LLM is available
pub fn is_llm_available() -> bool {
    LLMClient::new().is_available()
}

/// Convenience function to get optimization suggestions from profile JSON
pub fn suggest_optimizations(profile_json: &str) -> Result<LLMAnalysis, Box<dyn Error>> {
    LLMClient::new().analyze_profile(profile_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_client_creation() {
        let client = LLMClient::new();
        assert_eq!(client.model, "tinyllama");
    }

    #[test]
    fn test_extract_suggestions() {
        let client = LLMClient::new();
        let response = "Consider loop unrolling and dead code elimination for this hot code path";
        let suggestions = client.extract_suggestions(response);
        
        assert!(suggestions.len() >= 2);
        assert!(suggestions.iter().any(|s| s.strategy == "unroll_loop"));
        assert!(suggestions.iter().any(|s| s.strategy == "dead_code_elimination"));
    }
}
