use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use parking_lot::Mutex;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use super::types::{AudioError, SAMPLE_RATE};

pub struct MicCapture {
    pub _stream: Stream,
    pub consumer: Arc<Mutex<Consumer<f32>>>,
    /// Latest RMS as bits of f32, for UI level meter.
    pub rms_bits: Arc<AtomicU32>,
}

impl MicCapture {
    /// Start capturing from `device`. Returns a handle whose `consumer` is the audio thread's source.
    pub fn start(device: &Device) -> Result<Self, AudioError> {
        let config = device
            .default_input_config()
            .map_err(|e| AudioError::Cpal(e.to_string()))?;
        let in_sr = config.sample_rate().0;
        let in_channels = config.channels();
        let sample_format = config.sample_format();

        let (mut producer, consumer) = RingBuffer::<f32>::new(48_000 / 5);
        let consumer = Arc::new(Mutex::new(consumer));
        let rms_bits = Arc::new(AtomicU32::new(0));
        let rms_clone = rms_bits.clone();

        let stream_config = StreamConfig {
            channels: in_channels,
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        push_resampled(data, in_channels, in_sr, &mut producer, &rms_clone);
                    },
                    |err| eprintln!("mic stream error: {err}"),
                    None,
                )
                .map_err(|e| AudioError::Cpal(e.to_string()))?,

            SampleFormat::I16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        let f: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        push_resampled(&f, in_channels, in_sr, &mut producer, &rms_clone);
                    },
                    |err| eprintln!("mic stream error: {err}"),
                    None,
                )
                .map_err(|e| AudioError::Cpal(e.to_string()))?,

            SampleFormat::U16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _| {
                        let f: Vec<f32> =
                            data.iter()
                                .map(|&s| (s as f32 - 32768.0) / 32768.0)
                                .collect();
                        push_resampled(&f, in_channels, in_sr, &mut producer, &rms_clone);
                    },
                    |err| eprintln!("mic stream error: {err}"),
                    None,
                )
                .map_err(|e| AudioError::Cpal(e.to_string()))?,
        };

        stream.play().map_err(|e| AudioError::Cpal(e.to_string()))?;
        Ok(Self {
            _stream: stream,
            consumer,
            rms_bits,
        })
    }
}

fn push_resampled(
    data: &[f32],
    channels: u16,
    in_sr: u32,
    producer: &mut Producer<f32>,
    rms_atom: &AtomicU32,
) {
    let mono: Vec<f32> = if channels == 1 {
        data.to_vec()
    } else {
        data.chunks(channels as usize)
            .map(|c| c.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    let resampled: Vec<f32> = if in_sr == SAMPLE_RATE {
        mono
    } else {
        linear_resample(&mono, in_sr, SAMPLE_RATE)
    };

    let rms = if resampled.is_empty() {
        0.0
    } else {
        let s: f32 = resampled.iter().map(|x| x * x).sum();
        (s / resampled.len() as f32).sqrt()
    };
    rms_atom.store(rms.to_bits(), Ordering::Relaxed);

    for s in resampled {
        if producer.push(s).is_err() {
            break;
        }
    }
}

fn linear_resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let out_len = (input.len() as f64 * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let lo = src.floor() as usize;
        let hi = (lo + 1).min(input.len() - 1);
        let frac = (src - lo as f64) as f32;
        let a = input.get(lo).copied().unwrap_or(0.0);
        let b = input.get(hi).copied().unwrap_or(0.0);
        out.push(a + (b - a) * frac);
    }
    out
}

pub fn linear_resample_pub(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    linear_resample(input, from, to)
}

pub fn pull_frame(consumer: &Mutex<Consumer<f32>>, out: &mut [f32]) {
    let mut c = consumer.lock();
    for slot in out.iter_mut() {
        *slot = c.pop().unwrap_or(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_resample_identity() {
        let input = vec![1.0_f32, 2.0, 3.0];
        let out = linear_resample(&input, 48000, 48000);
        assert_eq!(out, input);
    }

    #[test]
    fn linear_resample_doubles_length() {
        let input = vec![0.0_f32, 1.0];
        let out = linear_resample(&input, 24000, 48000);
        assert!(out.len() >= 3);
    }
}
