use reqwest::multipart;
use std::sync::OnceLock;

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("reqwest client")
    })
}

pub async fn transcribe(
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
    language: &str,
    prompt: &str,
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

    let resp = client()
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Transcription timed out".into()
            } else if e.is_connect() {
                "Cannot reach OpenAI (network)".into()
            } else {
                format!("Network error: {e}")
            }
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(240).collect();
        return Err(match status.as_u16() {
            401 => "Invalid API key".into(),
            429 => "Rate limited by OpenAI".into(),
            _ => format!("API error {status}: {snippet}"),
        });
    }

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(text.trim().to_string())
}
