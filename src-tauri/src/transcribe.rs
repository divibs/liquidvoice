use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::multipart;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::config::is_qwen_model;

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::builder().build().expect("reqwest client"))
}

/// Long takes need longer than a fixed 30 s (upload + queue + decode).
fn timeout_for(duration_sec: u32) -> std::time::Duration {
    std::time::Duration::from_secs((30 + duration_sec).min(120) as u64)
}

/// Retry once on transport-layer failures (timeout, connect, request error).
/// HTTP status errors are not retried. Falls back to the original error when
/// the body is not cloneable (e.g. streaming multipart).
async fn send_with_retry(req: reqwest::RequestBuilder) -> Result<reqwest::Response, reqwest::Error> {
    let retry = req.try_clone();
    match req.send().await {
        Ok(resp) => Ok(resp),
        Err(e) if e.is_timeout() || e.is_connect() || e.is_request() => match retry {
            Some(retry) => {
                tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                retry.send().await
            }
            None => Err(e),
        },
        Err(e) => Err(e),
    }
}

pub async fn transcribe(
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
    language: &str,
    prompt: &str,
    duration_sec: u32,
) -> Result<String, String> {
    if is_qwen_model(model) {
        transcribe_qwen(api_key, model, wav_data, language, prompt, duration_sec).await
    } else {
        transcribe_openai(api_key, model, wav_data, language, prompt, duration_sec).await
    }
}

async fn transcribe_openai(
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
    language: &str,
    prompt: &str,
    duration_sec: u32,
) -> Result<String, String> {
    let mut form = multipart::Form::new()
        .part(
            "file",
            multipart::Part::bytes(wav_data)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| format!("multipart: {e}"))?,
        )
        .text("model", model.to_string())
        .text("response_format", "text");

    if !language.is_empty() {
        form = form.text("language", language.to_string());
    }
    if !prompt.is_empty() {
        form = form.text("prompt", prompt.to_string());
    }

    let resp = send_with_retry(
        client()
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(api_key)
            .multipart(form)
            .timeout(timeout_for(duration_sec)),
    )
    .await
    .map_err(map_network_err("OpenAI"))?;

    if !resp.status().is_success() {
        return Err(map_http_err("OpenAI", resp).await);
    }

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(text.trim().to_string())
}

async fn transcribe_qwen(
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
    language: &str,
    prompt: &str,
    duration_sec: u32,
) -> Result<String, String> {
    // Match the HTML tester success case: intl + input_audio + 16 kHz WAV data URL.
    let data_url = format!("data:audio/wav;base64,{}", B64.encode(&wav_data));

    // Vocabulary: system text (DashScope) + user input_text before audio (context enhancement).
    let mut messages = Vec::new();
    if !prompt.is_empty() {
        messages.push(json!({
            "role": "system",
            "content": [{ "text": prompt }]
        }));
        messages.push(json!({
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": prompt
            }]
        }));
    }
    messages.push(json!({
        "role": "user",
        "content": [{
            "type": "input_audio",
            "input_audio": { "data": data_url }
        }]
    }));

    let mut parameters = json!({
        "format": "wav",
        "sample_rate": "16000"
    });
    if !language.is_empty() {
        parameters["language"] = json!(language);
    }

    let body = json!({
        "model": model,
        "input": { "messages": messages },
        "parameters": parameters
    });

    let resp = send_with_retry(
        client()
            .post("https://dashscope-intl.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation")
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .header("X-DashScope-SSE", "disable")
            .json(&body)
            .timeout(timeout_for(duration_sec)),
    )
    .await
    .map_err(map_network_err("Qwen"))?;

    let status = resp.status();
    let body_text = resp.text().await.map_err(|e| format!("Qwen read: {e}"))?;

    if !status.is_success() {
        if body_text.contains("ASR_RESPONSE_HAVE_NO_WORDS") {
            return Ok(String::new());
        }
        let snippet: String = body_text.chars().take(400).collect();
        return Err(match status.as_u16() {
            401 | 403 => "Invalid Qwen API key (use DashScope intl key)".into(),
            429 => "Rate limited by Qwen".into(),
            _ => format!("Qwen API error {status}: {snippet}"),
        });
    }

    let value: Value =
        serde_json::from_str(&body_text).map_err(|e| format!("Qwen decode: {e}"))?;
    if let Some(text) = extract_qwen_text(&value) {
        return Ok(text);
    }

    // 200 but unexpected shape: surface a short body so it is not a silent skip.
    let snippet: String = body_text.chars().take(280).collect();
    Err(format!("Qwen returned no transcript text: {snippet}"))
}

fn extract_qwen_text(value: &Value) -> Option<String> {
    // DashScope multimodal: output.choices[0].message.content[{text}]
    if let Some(choices) = value.pointer("/output/choices").and_then(|v| v.as_array()) {
        if let Some(content) = choices
            .first()
            .and_then(|c| c.pointer("/message/content"))
        {
            if let Some(text) = content_to_text(content) {
                return Some(text);
            }
        }
    }
    // OpenAI-compatible: choices[0].message.content
    if let Some(content) = value.pointer("/choices/0/message/content") {
        if let Some(text) = content_to_text(content) {
            return Some(text);
        }
    }
    // Rare flat forms
    for path in ["/output/text", "/text"] {
        if let Some(s) = value.pointer(path).and_then(|v| v.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn content_to_text(content: &Value) -> Option<String> {
    if let Some(s) = content.as_str() {
        let t = s.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Some(arr) = content.as_array() {
        let text = arr
            .iter()
            .filter_map(|c| {
                c.get("text")
                    .and_then(|t| t.as_str())
                    .or_else(|| c.as_str())
            })
            .collect::<Vec<_>>()
            .join("");
        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

fn map_network_err(provider: &'static str) -> impl Fn(reqwest::Error) -> String {
    move |e| {
        if e.is_timeout() {
            format!("{provider} transcription timed out")
        } else if e.is_connect() {
            format!("Cannot reach {provider} (network)")
        } else {
            format!("Network error: {e}")
        }
    }
}

async fn map_http_err(provider: &str, resp: reqwest::Response) -> String {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let snippet: String = body.chars().take(400).collect();
    match status.as_u16() {
        401 | 403 => format!("Invalid {provider} API key"),
        429 => format!("Rate limited by {provider}"),
        _ => format!("{provider} API error {status}: {snippet}"),
    }
}
