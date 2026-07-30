use reqwest::multipart;

pub async fn transcribe(
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
    language: &str,
    prompt: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();

    let mut form = multipart::Form::new()
        .part(
            "file",
            multipart::Part::bytes(wav_data)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| e.to_string())?,
        )
        .text("model", model.to_string())
        .text("response_format", "text");

    if !language.is_empty() {
        form = form.text("language", language.to_string());
    }
    if !prompt.is_empty() {
        form = form.text("prompt", prompt.to_string());
    }

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API error {status}: {body}"));
    }

    resp.text().await.map_err(|e| e.to_string())
}
