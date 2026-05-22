use thiserror::Error;

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME_SAMPLES: usize = 480; // 10 ms @ 48 kHz mono

pub type Sample = f32;

#[derive(Debug, Clone)]
pub struct Frame(pub Vec<Sample>);

impl Frame {
    pub fn silent(n: usize) -> Self {
        Frame(vec![0.0; n])
    }
    pub fn rms(&self) -> f32 {
        if self.0.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = self.0.iter().map(|s| s * s).sum();
        (sum_sq / self.0.len() as f32).sqrt()
    }
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("device not found: {0}")]
    DeviceNotFound(String),
    #[error("cpal: {0}")]
    Cpal(String),
    #[error("decode: {0}")]
    Decode(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
