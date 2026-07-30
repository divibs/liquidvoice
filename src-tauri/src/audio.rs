use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

struct StreamWrapper(Option<cpal::Stream>);
unsafe impl Send for StreamWrapper {}

pub struct AudioRecorder {
    buffer: Arc<Mutex<Vec<i16>>>,
    stream: StreamWrapper,
    capture_rate: u32,
    /// Adaptive ambient floor for the live waveform meter only (0..1).
    noise_floor: Arc<Mutex<f32>>,
}

unsafe impl Send for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            stream: StreamWrapper(None),
            capture_rate: 48000,
            noise_floor: Arc::new(Mutex::new(0.008)),
        }
    }

    pub fn start(&mut self, on_level: impl Fn(f32) + Send + 'static) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No default input device")?;

        // Use device's default config (usually 48kHz stereo on Windows)
        let config = device
            .default_input_config()
            .map_err(|e| format!("Can't get default mic config: {e}"))?;

        self.capture_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        self.buffer.lock().unwrap().clear();
        *self.noise_floor.lock().unwrap() = 0.008;
        let buffer = self.buffer.clone();
        let noise_floor = self.noise_floor.clone();

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    // Mix down to mono if stereo
                    let mono: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| {
                                chunk.iter().map(|&s| s as i32).sum::<i32>() as i16
                                    / channels as i16
                            })
                            .collect()
                    } else {
                        data.to_vec()
                    };

                    buffer.lock().unwrap().extend_from_slice(&mono);

                    let rms = (mono.iter().map(|&s| (s as f32).powi(2)).sum::<f32>()
                        / mono.len().max(1) as f32)
                        .sqrt()
                        / 32768.0;

                    // UI-only: track ambient (fan) floor; does not alter recorded PCM.
                    let mut floor = noise_floor.lock().unwrap();
                    if rms < *floor * 2.2 {
                        *floor = *floor * 0.9 + rms * 0.1;
                    }
                    *floor = floor.clamp(0.003, 0.08);

                    // Less sensitive waveform — need clear headroom above ambient.
                    let gated = ((rms - *floor * 3.2) / 0.4).clamp(0.0, 1.0);
                    on_level(gated * gated);
                },
                |err| eprintln!("Audio stream error: {err}"),
                None,
            )
            .map_err(|e| format!("Can't open mic stream: {e}"))?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream.0 = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> Vec<i16> {
        self.stream.0 = None;
        let raw = self.buffer.lock().unwrap().clone();

        // Resample to 16kHz if captured at higher rate (no denoise — raw audio to API).
        if self.capture_rate != 16000 {
            resample(&raw, self.capture_rate, 16000)
        } else {
            raw
        }
    }
}

fn resample(input: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if input.is_empty() {
        return vec![];
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (input.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;

        let sample = if idx + 1 < input.len() {
            let a = input[idx] as f64;
            let b = input[idx + 1] as f64;
            (a + frac * (b - a)) as i16
        } else if idx < input.len() {
            input[idx]
        } else {
            0
        };
        output.push(sample);
    }
    output
}

pub fn pcm_to_wav(pcm: &[i16], sample_rate: u32) -> Result<Vec<u8>, String> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|e| e.to_string())?;
    for &sample in pcm {
        writer.write_sample(sample).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

/// True if the buffer has enough energetic frames to count as real speech.
/// Used to skip the API on silence (avoids Whisper filler hallucinations).
pub fn has_audible_speech(pcm: &[i16], sample_rate: u32) -> bool {
    if pcm.len() < (sample_rate as usize) / 5 {
        return false; // < ~200 ms
    }
    let frame = ((sample_rate as usize) / 50).max(64); // ~20 ms
    let mut speech_frames = 0usize;
    let mut total = 0usize;
    let mut peak = 0.0_f32;

    for chunk in pcm.chunks(frame) {
        total += 1;
        let rms = (chunk.iter().map(|&s| (s as f32).powi(2)).sum::<f32>()
            / chunk.len() as f32)
            .sqrt()
            / 32768.0;
        peak = peak.max(rms);
        if rms > 0.025 {
            speech_frames += 1;
        }
    }

    speech_frames >= 5 && peak > 0.045 && (speech_frames as f32 / total.max(1) as f32) > 0.05
}

/// Filter Whisper-style silence hallucinations when the model invents filler.
pub fn is_likely_hallucination(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return true;
    }
    const PHRASES: &[&str] = &[
        "thank you",
        "thanks for watching",
        "thanks for listening",
        "thank you for watching",
        "please subscribe",
        "字幕",
        "字幕by",
        "amara.org",
        "www.youtube.com",
        "you",
        ".",
        "...",
        "bye",
        "bye.",
        "okay",
        "ok",
        "um",
        "uh",
        "hmm",
        "mbc 뉴스",
        "시청해 주셔서 감사합니다",
    ];
    let stripped: String = t
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if stripped.chars().count() <= 2 {
        return true;
    }
    PHRASES.iter().any(|p| stripped == *p)
}
