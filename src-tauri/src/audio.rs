use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

struct StreamWrapper(Option<cpal::Stream>);
unsafe impl Send for StreamWrapper {}

pub struct AudioRecorder {
    buffer: Arc<Mutex<Vec<i16>>>,
    stream: StreamWrapper,
}

unsafe impl Send for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            stream: StreamWrapper(None),
        }
    }

    pub fn start(&mut self, on_level: impl Fn(f32) + Send + 'static) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No default input device")?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16000),
            buffer_size: cpal::BufferSize::Default,
        };

        self.buffer.lock().unwrap().clear();
        let buffer = self.buffer.clone();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    buffer.lock().unwrap().extend_from_slice(data);
                    let rms = (data.iter().map(|&s| (s as f32).powi(2)).sum::<f32>()
                        / data.len().max(1) as f32)
                        .sqrt()
                        / 32768.0;
                    on_level(rms);
                },
                |err| eprintln!("Audio stream error: {err}"),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream.0 = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> Vec<i16> {
        self.stream.0 = None;
        self.buffer.lock().unwrap().clone()
    }
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
