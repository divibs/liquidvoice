use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

struct StreamWrapper(Option<cpal::Stream>);
unsafe impl Send for StreamWrapper {}

pub struct AudioRecorder {
    buffer: Arc<Mutex<Vec<i16>>>,
    stream: StreamWrapper,
    capture_rate: u32,
}

unsafe impl Send for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            stream: StreamWrapper(None),
            capture_rate: 48000,
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
        let buffer = self.buffer.clone();

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    // Mix down to mono if stereo
                    let mono: Vec<i16> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| chunk.iter().map(|&s| s as i32).sum::<i32>() as i16 / channels as i16)
                            .collect()
                    } else {
                        data.to_vec()
                    };

                    buffer.lock().unwrap().extend_from_slice(&mono);

                    let rms = (mono.iter().map(|&s| (s as f32).powi(2)).sum::<f32>()
                        / mono.len().max(1) as f32)
                        .sqrt()
                        / 32768.0;
                    on_level(rms);
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

        // Resample to 16kHz if captured at higher rate
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
