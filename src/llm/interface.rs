use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|err| format!("failed to create HTTP client: {err}"))?;

        Ok(Self {
            client,
            base_url: base_url.into(),
            model: model.into(),
        })
    }

    pub fn local(model: impl Into<String>) -> Result<Self, String> {
        Self::new("http://127.0.0.1:11434", model)
    }

    pub fn generate(&self, prompt: &str) -> Result<String, String> {
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let body = GenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
        };

        let response = self
            .client
            .post(url)
            .json(&body)
            .send()
            .map_err(|err| format!("request to Ollama failed: {err}"))?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .unwrap_or_else(|_| "<unable to read response body>".to_string());
            return Err(format!("Ollama returned {status}: {text}"));
        }

        let payload: GenerateResponse = response
            .json()
            .map_err(|err| format!("invalid Ollama response payload: {err}"))?;

        Ok(payload.response.trim().to_string())
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}
