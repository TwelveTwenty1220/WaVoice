use parking_lot::Mutex;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::sample::Sample as SymphSample;

use super::mic_capture::linear_resample_pub;
use super::types::{AudioError, SAMPLE_RATE};

/// One playing voice — either looping music or a one-shot SFX.
pub struct Voice {
    pub samples: Vec<f32>,
    pub position: usize,
    pub looping: bool,
    pub gain: f32,
}

impl Voice {
    pub fn pull(&mut self, out: &mut [f32]) {
        for slot in out.iter_mut() {
            if self.position >= self.samples.len() {
                if self.looping {
                    self.position = 0;
                } else {
                    return;
                }
            }
            *slot += self.samples[self.position] * self.gain;
            self.position += 1;
        }
    }
    pub fn finished(&self) -> bool {
        !self.looping && self.position >= self.samples.len()
    }
}

#[derive(Default)]
pub struct MusicPlayer {
    voices: Vec<Voice>,
}

impl MusicPlayer {
    pub fn play(&mut self, samples: Vec<f32>, looping: bool, gain: f32) {
        if self.voices.len() >= 8 {
            self.voices.remove(0);
        }
        self.voices.push(Voice {
            samples,
            position: 0,
            looping,
            gain,
        });
    }
    pub fn stop_all(&mut self) {
        self.voices.clear();
    }
    pub fn mix_into(&mut self, out: &mut [f32]) {
        for slot in out.iter_mut() {
            *slot = 0.0;
        }
        for v in self.voices.iter_mut() {
            v.pull(out);
        }
        self.voices.retain(|v| !v.finished());
    }
}

pub type SharedPlayer = Arc<Mutex<MusicPlayer>>;

/// Decode a file fully into mono 48 kHz f32 samples.
pub fn decode_to_mono_48k(path: &Path) -> Result<Vec<f32>, AudioError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AudioError::Decode(e.to_string()))?;
    let mut format: Box<dyn FormatReader> = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| AudioError::Decode("no audio track".into()))?;
    let track_id = track.id;
    let track_sr = track.codec_params.sample_rate.unwrap_or(SAMPLE_RATE);
    let track_channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(1);

    let mut decoder: Box<dyn Decoder> = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioError::Decode(e.to_string()))?;

    let mut mono: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(AudioError::Decode(e.to_string())),
        };
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => append_mono_f32(&decoded, track_channels, &mut mono),
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(AudioError::Decode(e.to_string())),
        }
    }

    if track_sr != SAMPLE_RATE {
        mono = linear_resample_pub(&mono, track_sr, SAMPLE_RATE);
    }
    Ok(mono)
}

fn append_mono_f32(buf: &AudioBufferRef<'_>, channels: u16, out: &mut Vec<f32>) {
    use AudioBufferRef::*;
    match buf {
        F32(b) => {
            let frames = b.frames();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..(channels as usize).min(b.spec().channels.count()) {
                    sum += b.chan(ch)[i];
                }
                out.push(sum / channels as f32);
            }
        }
        S16(b) => push_int(b, channels, out, |s: i16| s as f32 / i16::MAX as f32),
        S32(b) => push_int(b, channels, out, |s: i32| s as f32 / i32::MAX as f32),
        U8(b) => push_int(b, channels, out, |s: u8| (s as f32 - 128.0) / 128.0),
        U16(b) => push_int(b, channels, out, |s: u16| (s as f32 - 32768.0) / 32768.0),
        U32(b) => push_int(b, channels, out, |s: u32| {
            (s as f32 - 2_147_483_648.0) / 2_147_483_648.0
        }),
        S8(b) => push_int(b, channels, out, |s: i8| s as f32 / i8::MAX as f32),
        F64(b) => {
            let frames = b.frames();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..(channels as usize).min(b.spec().channels.count()) {
                    sum += b.chan(ch)[i] as f32;
                }
                out.push(sum / channels as f32);
            }
        }
        S24(b) => {
            let frames = b.frames();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..(channels as usize).min(b.spec().channels.count()) {
                    let v = b.chan(ch)[i].inner();
                    sum += v as f32 / (1 << 23) as f32;
                }
                out.push(sum / channels as f32);
            }
        }
        U24(b) => {
            let frames = b.frames();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..(channels as usize).min(b.spec().channels.count()) {
                    let v = b.chan(ch)[i].inner();
                    sum += (v as f32 - (1 << 23) as f32) / (1 << 23) as f32;
                }
                out.push(sum / channels as f32);
            }
        }
    }
}

fn push_int<T: Copy + SymphSample>(
    b: &symphonia::core::audio::AudioBuffer<T>,
    channels: u16,
    out: &mut Vec<f32>,
    norm: impl Fn(T) -> f32,
) {
    let frames = b.frames();
    for i in 0..frames {
        let mut sum = 0.0f32;
        for ch in 0..(channels as usize).min(b.spec().channels.count()) {
            let val = *b.chan(ch).get(i).unwrap_or(&T::MID);
            sum += norm(val);
        }
        out.push(sum / channels as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_pull_loops() {
        let mut v = Voice {
            samples: vec![1.0, 2.0, 3.0],
            position: 0,
            looping: true,
            gain: 1.0,
        };
        let mut out = [0.0_f32; 5];
        v.pull(&mut out);
        assert_eq!(out, [1.0, 2.0, 3.0, 1.0, 2.0]);
    }

    #[test]
    fn voice_pull_oneshot_finishes() {
        let mut v = Voice {
            samples: vec![1.0, 2.0],
            position: 0,
            looping: false,
            gain: 1.0,
        };
        let mut out = [0.0_f32; 5];
        v.pull(&mut out);
        assert_eq!(out, [1.0, 2.0, 0.0, 0.0, 0.0]);
        assert!(v.finished());
    }

    #[test]
    fn player_mixes_multiple_voices() {
        let mut p = MusicPlayer::default();
        p.play(vec![0.3; 10], false, 1.0);
        p.play(vec![0.4; 10], false, 1.0);
        let mut out = [0.0_f32; 5];
        p.mix_into(&mut out);
        for s in &out {
            approx::assert_abs_diff_eq!(*s, 0.7, epsilon = 1e-5);
        }
    }
}
