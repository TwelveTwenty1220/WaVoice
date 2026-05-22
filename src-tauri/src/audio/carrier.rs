use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;

use super::music_player::decode_to_mono_48k;
use super::types::AudioError;

#[derive(Default)]
pub struct Carrier {
    samples: Vec<f32>,
    position: usize,
}

impl Carrier {
    pub fn load_file(&mut self, path: &Path) -> Result<(), AudioError> {
        self.samples = decode_to_mono_48k(path)?;
        self.position = 0;
        Ok(())
    }
    pub fn loaded(&self) -> bool {
        !self.samples.is_empty()
    }
    /// Fill `out` with carrier samples, looping. If unloaded, output is silence.
    pub fn pull(&mut self, out: &mut [f32]) {
        if self.samples.is_empty() {
            for slot in out.iter_mut() {
                *slot = 0.0;
            }
            return;
        }
        for slot in out.iter_mut() {
            *slot = self.samples[self.position];
            self.position = (self.position + 1) % self.samples.len();
        }
    }
}

pub type SharedCarrier = Arc<Mutex<Carrier>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unloaded_carrier_emits_silence() {
        let mut c = Carrier::default();
        let mut out = [0.5_f32; 10];
        c.pull(&mut out);
        assert!(out.iter().all(|&s| s == 0.0));
    }
}
