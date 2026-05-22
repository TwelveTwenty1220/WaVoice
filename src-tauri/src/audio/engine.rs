use cpal::traits::HostTrait;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use super::carrier::{Carrier, SharedCarrier};
use super::mic_capture::{pull_frame, MicCapture};
use super::mixer::{mix, MixGains, MixInput};
use super::music_player::{MusicPlayer, SharedPlayer};
use super::types::AudioError;
use super::virtual_sink::{find_input_device, find_output_device, VirtualSink};

pub struct AudioEngine {
    pub gains: Arc<Mutex<MixGains>>,
    pub mic_rms_bits: Arc<AtomicU32>,
    pub output_rms_bits: Arc<AtomicU32>,
    pub player: SharedPlayer,
    pub carrier: SharedCarrier,
    pub running: Arc<AtomicBool>,
    pub _mic: MicCapture,
    pub _sink: VirtualSink,
}

impl AudioEngine {
    pub fn start(input_device_name: &str, output_device_name: &str) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let input = find_input_device(&host, input_device_name)
            .ok_or_else(|| AudioError::DeviceNotFound(input_device_name.into()))?;
        let output = find_output_device(&host, output_device_name)
            .ok_or_else(|| AudioError::DeviceNotFound(output_device_name.into()))?;

        let mic = MicCapture::start(&input)?;
        let player: SharedPlayer = Arc::new(Mutex::new(MusicPlayer::default()));
        let carrier: SharedCarrier = Arc::new(Mutex::new(Carrier::default()));
        let gains = Arc::new(Mutex::new(MixGains::default()));
        let output_rms = Arc::new(AtomicU32::new(0));
        let running = Arc::new(AtomicBool::new(true));

        let mic_consumer = mic.consumer.clone();
        let player_for_thread = player.clone();
        let carrier_for_thread = carrier.clone();
        let gains_for_thread = gains.clone();
        let output_rms_for_thread = output_rms.clone();
        let running_for_thread = running.clone();

        let sink = VirtualSink::start(&output, move |out_mono: &mut [f32]| {
            if !running_for_thread.load(Ordering::Relaxed) {
                for s in out_mono.iter_mut() {
                    *s = 0.0;
                }
                return;
            }
            let n = out_mono.len();
            let mut mic_buf = vec![0.0f32; n];
            let mut music_buf = vec![0.0f32; n];
            let mut carrier_buf = vec![0.0f32; n];

            pull_frame(&mic_consumer, &mut mic_buf);
            player_for_thread.lock().mix_into(&mut music_buf);
            carrier_for_thread.lock().pull(&mut carrier_buf);

            let g = *gains_for_thread.lock();
            let mixed = mix(
                MixInput {
                    mic: &mic_buf,
                    music: &music_buf,
                    carrier: &carrier_buf,
                },
                &g,
            );
            out_mono.copy_from_slice(&mixed.0);

            let mut sum_sq = 0.0f32;
            for s in out_mono.iter() {
                sum_sq += s * s;
            }
            let rms = (sum_sq / n as f32).sqrt();
            output_rms_for_thread.store(rms.to_bits(), Ordering::Relaxed);
        })?;

        Ok(Self {
            gains,
            mic_rms_bits: mic.rms_bits.clone(),
            output_rms_bits: output_rms,
            player,
            carrier,
            running,
            _mic: mic,
            _sink: sink,
        })
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.player.lock().stop_all();
    }
}
