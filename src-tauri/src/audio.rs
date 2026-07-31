use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::sync::{Arc, Mutex};

struct StreamWrapper(Option<cpal::Stream>);
// cpal::Stream is !Send on some platforms; we only ever touch it from the
// Tauri main/event thread after stopping the callback.
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
        // Drop any previous stream before opening a new one.
        self.stream.0 = None;

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No default input device".to_string())?;

        // Use device default config (often 48 kHz stereo on Windows).
        // Never request 16 kHz directly; many Windows mics reject it.
        let supported = device
            .default_input_config()
            .map_err(|e| format!("Can't get default mic config: {e}"))?;

        self.capture_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();

        self.buffer
            .lock()
            .map_err(|_| "audio buffer lock poisoned".to_string())?
            .clear();
        *self
            .noise_floor
            .lock()
            .map_err(|_| "noise floor lock poisoned".to_string())? = 0.008;

        let buffer = self.buffer.clone();
        let noise_floor = self.noise_floor.clone();

        let err_fn = |err| eprintln!("Audio stream error: {err}");

        let stream = match sample_format {
            SampleFormat::I16 => device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        push_samples(&buffer, &noise_floor, &on_level, channels, data.iter().copied());
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Can't open mic stream (i16): {e}"))?,
            SampleFormat::F32 => device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let samples = data.iter().map(|&s| {
                            (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
                        });
                        push_samples(&buffer, &noise_floor, &on_level, channels, samples);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Can't open mic stream (f32): {e}"))?,
            SampleFormat::U16 => device
                .build_input_stream(
                    &config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let samples = data.iter().map(|&s| {
                            ((s as i32) - 32768).clamp(i16::MIN as i32, i16::MAX as i32) as i16
                        });
                        push_samples(&buffer, &noise_floor, &on_level, channels, samples);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Can't open mic stream (u16): {e}"))?,
            other => {
                return Err(format!("Unsupported mic sample format: {other:?}"));
            }
        };

        stream.play().map_err(|e| format!("Mic play failed: {e}"))?;
        self.stream.0 = Some(stream);
        Ok(())
    }

    /// Stop capture and return 16 kHz mono PCM, truncated to `max_sec` if set.
    pub fn stop(&mut self, max_sec: Option<u32>) -> Vec<i16> {
        self.stream.0 = None;
        let raw = self
            .buffer
            .lock()
            .map(|b| b.clone())
            .unwrap_or_default();

        let mut pcm = if self.capture_rate != 16_000 {
            resample(&raw, self.capture_rate, 16_000)
        } else {
            raw
        };

        if let Some(sec) = max_sec {
            let max_samples = 16_000u64.saturating_mul(sec as u64) as usize;
            if pcm.len() > max_samples {
                pcm.truncate(max_samples);
            }
        }
        pcm
    }
}

fn push_samples(
    buffer: &Arc<Mutex<Vec<i16>>>,
    noise_floor: &Arc<Mutex<f32>>,
    on_level: &impl Fn(f32),
    channels: usize,
    samples: impl Iterator<Item = i16>,
) {
    let interleaved: Vec<i16> = samples.collect();
    let mono = mix_to_mono(&interleaved, channels);

    if let Ok(mut buf) = buffer.lock() {
        buf.extend_from_slice(&mono);
    }

    if mono.is_empty() {
        return;
    }

    let rms = (mono.iter().map(|&s| (s as f32).powi(2)).sum::<f32>() / mono.len() as f32).sqrt()
        / 32768.0;

    // UI-only ambient floor (fan); does not alter recorded PCM.
    if let Ok(mut floor) = noise_floor.lock() {
        if rms < *floor * 2.0 {
            *floor = *floor * 0.92 + rms * 0.08;
        }
        *floor = floor.clamp(0.002, 0.06);

        // Stronger gate: quiet ambient stays flat; normal speech still moves bars.
        let gated = ((rms - *floor * 3.0) / 0.28).clamp(0.0, 1.0);
        on_level(gated);
    }
}

fn mix_to_mono(interleaved: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(channels)
        .map(|chunk| {
            let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
            (sum / channels as i32) as i16
        })
        .collect()
}

pub fn resample(input: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if input.is_empty() || from_rate == 0 || to_rate == 0 {
        return vec![];
    }
    if from_rate == to_rate {
        return input.to_vec();
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
            (a + frac * (b - a)).round() as i16
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
/// Thresholds stay low so normal desk distance still counts as speech.
pub fn has_audible_speech(pcm: &[i16], sample_rate: u32) -> bool {
    if pcm.len() < (sample_rate as usize) / 8 {
        return false; // < ~125 ms
    }
    let frame = ((sample_rate as usize) / 50).max(64); // ~20 ms
    let mut speech_frames = 0usize;
    let mut total = 0usize;
    let mut peak = 0.0_f32;
    let mut sum_rms = 0.0_f32;

    for chunk in pcm.chunks(frame) {
        total += 1;
        let rms = (chunk.iter().map(|&s| (s as i32 as f32).powi(2)).sum::<f32>()
            / chunk.len() as f32)
            .sqrt()
            / 32768.0;
        peak = peak.max(rms);
        sum_rms += rms;
        // Soft voice at desk distance often sits around 0.01-0.03 RMS.
        if rms > 0.008 {
            speech_frames += 1;
        }
    }

    let mean = sum_rms / total.max(1) as f32;
    let speech_ratio = speech_frames as f32 / total.max(1) as f32;
    speech_frames >= 3 && peak > 0.015 && (mean > 0.004 || speech_ratio > 0.03)
}

/// Filter Whisper-style silence hallucinations when the model invents filler.
/// Keep this conservative: never drop short real words like "ok", "you", "bye".
pub fn is_likely_hallucination(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return true;
    }

    let stripped: String = t
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if stripped.is_empty() {
        return true;
    }

    // Punctuation-only / tiny noise after strip.
    if stripped.chars().count() <= 1 {
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
        "mbc 뉴스",
        "시청해 주셔서 감사합니다",
    ];
    PHRASES.iter().any(|p| stripped == *p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_mono_averages_channels() {
        let stereo = vec![1000_i16, 3000, -2000, 2000];
        assert_eq!(mix_to_mono(&stereo, 2), vec![2000, 0]);
    }

    #[test]
    fn resample_identity_and_empty() {
        let pcm = vec![1, 2, 3, 4];
        assert_eq!(resample(&pcm, 16000, 16000), pcm);
        assert!(resample(&[], 48000, 16000).is_empty());
        assert!(resample(&pcm, 0, 16000).is_empty());
    }

    #[test]
    fn resample_downsamples_length() {
        let pcm = vec![0_i16; 48000];
        let out = resample(&pcm, 48000, 16000);
        assert!((out.len() as i32 - 16000).abs() <= 1);
    }

    #[test]
    fn silence_is_not_speech() {
        let pcm = vec![0_i16; 16000];
        assert!(!has_audible_speech(&pcm, 16000));
    }

    #[test]
    fn loud_burst_counts_as_speech() {
        let mut pcm = vec![0_i16; 16000];
        for s in pcm.iter_mut().take(4000) {
            *s = 8000;
        }
        assert!(has_audible_speech(&pcm, 16000));
    }

    #[test]
    fn hallucination_filter_keeps_real_words() {
        assert!(!is_likely_hallucination("ok"));
        assert!(!is_likely_hallucination("you"));
        assert!(!is_likely_hallucination("bye"));
        assert!(!is_likely_hallucination("hello world"));
        assert!(is_likely_hallucination("thank you"));
        assert!(is_likely_hallucination("Thanks for watching!"));
        assert!(is_likely_hallucination("..."));
        assert!(is_likely_hallucination(""));
    }

    #[test]
    fn wav_roundtrip_header() {
        let pcm = vec![0_i16, 100, -100, 50];
        let wav = pcm_to_wav(&pcm, 16000).unwrap();
        assert!(wav.starts_with(b"RIFF"));
        assert!(wav.len() > 44);
    }
}
