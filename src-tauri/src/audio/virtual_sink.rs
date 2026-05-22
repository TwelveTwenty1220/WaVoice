use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig};

use super::types::AudioError;

pub const VB_CABLE_INPUT_HINT: &str = "CABLE Input";

/// Enumerate output devices; user picks one (typically "CABLE Input (VB-Audio Virtual Cable)").
pub fn list_output_devices(host: &Host) -> Vec<String> {
    host.output_devices()
        .map(|iter| iter.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

pub fn list_input_devices(host: &Host) -> Vec<String> {
    host.input_devices()
        .map(|iter| iter.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

pub fn find_output_device(host: &Host, name: &str) -> Option<Device> {
    host.output_devices()
        .ok()?
        .find(|d| d.name().map(|n| n == name).unwrap_or(false))
}

pub fn find_input_device(host: &Host, name: &str) -> Option<Device> {
    host.input_devices()
        .ok()?
        .find(|d| d.name().map(|n| n == name).unwrap_or(false))
}

pub fn detect_vb_cable(host: &Host) -> Option<String> {
    list_output_devices(host)
        .into_iter()
        .find(|n| n.contains(VB_CABLE_INPUT_HINT))
}

pub struct VirtualSink {
    pub _stream: Stream,
}

impl VirtualSink {
    /// Start writing to `device`. `fill` is called from the audio thread for each output buffer.
    pub fn start<F>(device: &Device, mut fill: F) -> Result<Self, AudioError>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        let config = device
            .default_output_config()
            .map_err(|e| AudioError::Cpal(e.to_string()))?;
        let channels = config.channels() as usize;
        let sample_format = config.sample_format();
        let stream_config = StreamConfig {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _| {
                        let frames = data.len() / channels;
                        let mut mono = vec![0.0f32; frames];
                        fill(&mut mono);
                        for (i, frame) in data.chunks_mut(channels).enumerate() {
                            let s = mono[i];
                            for ch in frame.iter_mut() {
                                *ch = s;
                            }
                        }
                    },
                    |err| eprintln!("output stream error: {err}"),
                    None,
                )
                .map_err(|e| AudioError::Cpal(e.to_string()))?,

            SampleFormat::I16 => device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _| {
                        let frames = data.len() / channels;
                        let mut mono = vec![0.0f32; frames];
                        fill(&mut mono);
                        for (i, frame) in data.chunks_mut(channels).enumerate() {
                            let s = (mono[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            for ch in frame.iter_mut() {
                                *ch = s;
                            }
                        }
                    },
                    |err| eprintln!("output stream error: {err}"),
                    None,
                )
                .map_err(|e| AudioError::Cpal(e.to_string()))?,

            other => {
                return Err(AudioError::Cpal(format!(
                    "unsupported output format {:?}",
                    other
                )))
            }
        };

        stream.play().map_err(|e| AudioError::Cpal(e.to_string()))?;
        Ok(Self { _stream: stream })
    }
}
