# WaVoic MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a working Windows desktop app that lets a Valorant player play local music/SFX into their team voice chat reliably (no Krisp choppiness), using hotkeys and a simple library UI.

**Architecture:** Tauri shell (Rust backend + TS/HTML frontend). All audio I/O in Rust via `cpal` (WASAPI on Windows). A single audio thread runs a 10 ms mixer tick that combines (a) user's real mic, (b) decoded music, (c) optional pre-recorded voice carrier — and writes the mix to the VB-Cable virtual input device, which Valorant reads as the user's microphone. The live mic underneath the music defeats Valorant's Krisp gating by keeping VAD always-on.

**Tech Stack:** Tauri 2, Rust (cpal, symphonia, rubato, rtrb, global-hotkey, serde, parking_lot), TypeScript + vanilla DOM frontend.

**Environment note:** Dev environment is a Linux remote (PyCharm SSH). All real audio testing must happen on Windows. Code in this plan compiles on both — `cpal` selects ALSA on Linux, WASAPI on Windows. The VB-Cable device check is a string-match on device names.

---

## File Structure

**Rust backend (`src-tauri/src/`):**
- `main.rs` — Tauri app entry, command registration
- `state.rs` — shared `AppState` (Arc'd, holds engine handles + config)
- `config.rs` — load/save `config.json` (selected devices, hotkeys, gains)
- `audio/mod.rs` — re-exports + engine bootstrap
- `audio/types.rs` — common types (`Frame`, `EngineHandle`, control messages)
- `audio/mic_capture.rs` — input stream → ring buffer
- `audio/music_player.rs` — symphonia decoder + voice pool
- `audio/carrier.rs` — voice-carrier sample loader + RMS gate
- `audio/mixer.rs` — pure mixing math (testable without audio devices)
- `audio/engine.rs` — wires capture + player + mixer + sink on threads
- `audio/virtual_sink.rs` — output stream to VB-Cable
- `library.rs` — file index, metadata, persistence
- `hotkeys.rs` — global-hotkey wrapper
- `commands.rs` — Tauri IPC commands (frontend ↔ backend)

**Frontend (`src/`):**
- `index.html` — single page with tabs
- `main.ts` — bootstraps UI, subscribes to backend events
- `api.ts` — typed wrappers around `invoke()` calls
- `components/library.ts` — file list + drag-and-drop import
- `components/player-bar.ts` — transport + gain sliders + level meter
- `components/hotkeys.ts` — binding UI
- `components/settings.ts` — device pickers + Krisp-bypass toggle
- `components/wizard.ts` — first-run flow
- `styles.css`

**Docs:**
- `README.md` — setup (install Rust, install Node, install VB-Cable, build/run)
- `docs/troubleshooting.md` — common errors

---

## Task 0: Project Scaffold

**Files:**
- Create: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src/index.html`, `src/main.ts`, `vite.config.ts`, `tsconfig.json`, `README.md`

- [ ] **Step 1: Create `package.json`**

```json
{
  "name": "wavoic",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-dialog": "^2.0.0",
    "@tauri-apps/plugin-fs": "^2.0.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "typescript": "^5.4.0",
    "vite": "^5.2.0"
  }
}
```

- [ ] **Step 2: Create `vite.config.ts`**

```ts
import { defineConfig } from "vite";
export default defineConfig({
  root: "src",
  build: { outDir: "../dist", emptyOutDir: true },
  server: { port: 1420, strictPort: true, host: "127.0.0.1" },
  clearScreen: false,
});
```

- [ ] **Step 3: Create `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "skipLibCheck": true,
    "isolatedModules": true,
    "noEmit": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"]
  },
  "include": ["src/**/*.ts"]
}
```

- [ ] **Step 4: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "wavoic"
version = "0.1.0"
edition = "2021"
description = "Play music into Valorant voice chat"

[lib]
name = "wavoic_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
cpal = "0.15"
symphonia = { version = "0.5", features = ["all"] }
rubato = "0.15"
rtrb = "0.3"
global-hotkey = "0.6"
parking_lot = "0.12"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
once_cell = "1"

[dev-dependencies]
approx = "0.5"
```

- [ ] **Step 5: Create `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 6: Create minimal `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tracing_subscriber::fmt::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 7: Create `src-tauri/tauri.conf.json`**

```json
{
  "productName": "WaVoic",
  "version": "0.1.0",
  "identifier": "com.wavoic.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [{
      "title": "WaVoic",
      "width": 1000,
      "height": 700,
      "minWidth": 800,
      "minHeight": 600
    }],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.ico"]
  }
}
```

- [ ] **Step 8: Create skeleton `src/index.html`, `src/main.ts`, `src/styles.css`**

```html
<!doctype html>
<html lang="en"><head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>WaVoic</title>
  <link rel="stylesheet" href="styles.css" />
</head><body>
  <div id="app"></div>
  <script type="module" src="main.ts"></script>
</body></html>
```

```ts
// src/main.ts
const app = document.querySelector<HTMLDivElement>("#app")!;
app.innerHTML = `<h1>WaVoic</h1><p>Booting…</p>`;
```

- [ ] **Step 9: Create `README.md` with build instructions**

```md
# WaVoic

Play local music and SFX into your Valorant voice chat without Krisp choppiness.

## Prerequisites (Windows)
1. **Rust** — install via https://rustup.rs
2. **Node.js** ≥ 20 — install via https://nodejs.org
3. **VB-Cable** — install from https://vb-audio.com/Cable/ (reboot after)
4. **Visual Studio Build Tools** (C++) — required by Tauri on Windows

## Development
```
npm install
npm run tauri dev
```

## Build for release
```
npm run tauri build
```
Output: `src-tauri/target/release/bundle/nsis/WaVoic_*_x64-setup.exe`

## Valorant Setup
1. Open Valorant → Settings → Audio → Voice Chat.
2. Set **Input Device** to `CABLE Output (VB-Audio Virtual Cable)`.
3. In WaVoic Settings, pick your real mic as the input and `CABLE Input` as the output.
```

- [ ] **Step 10: Commit**

```bash
git add package.json vite.config.ts tsconfig.json src-tauri/ src/ README.md
git commit -m "scaffold: initial Tauri + Vite + Rust project structure"
```

---

## Task 1: Audio Types and Errors

**Files:**
- Create: `src-tauri/src/audio/mod.rs`, `src-tauri/src/audio/types.rs`

- [ ] **Step 1: Create `src-tauri/src/audio/mod.rs`**

```rust
pub mod types;
pub mod mic_capture;
pub mod music_player;
pub mod carrier;
pub mod mixer;
pub mod virtual_sink;
pub mod engine;
```

- [ ] **Step 2: Create `src-tauri/src/audio/types.rs` with shared types**

```rust
use thiserror::Error;

pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u16 = 1;
pub const FRAME_SAMPLES: usize = 480; // 10 ms @ 48 kHz mono

pub type Sample = f32;

#[derive(Debug, Clone)]
pub struct Frame(pub Vec<Sample>);

impl Frame {
    pub fn silent(n: usize) -> Self {
        Frame(vec![0.0; n])
    }
    pub fn rms(&self) -> f32 {
        if self.0.is_empty() { return 0.0; }
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
```

- [ ] **Step 3: Register `mod audio;` in `src-tauri/src/main.rs`**

Add `mod audio;` near the top of `main.rs`.

- [ ] **Step 4: Build and commit**

```bash
cd src-tauri && cargo check && cd ..
git add src-tauri/src/audio/mod.rs src-tauri/src/audio/types.rs src-tauri/src/main.rs
git commit -m "feat(audio): add shared audio types and error enum"
```

---

## Task 2: Mixer (Pure, Unit-Tested)

The mixer is the most testable piece — it's just math. Build it first, in isolation.

**Files:**
- Create: `src-tauri/src/audio/mixer.rs`
- Modify: nothing yet

- [ ] **Step 1: Write the failing test for plain summation**

Create `src-tauri/src/audio/mixer.rs`:

```rust
use super::types::{Frame, Sample};

pub struct MixInput<'a> {
    pub mic: &'a [Sample],
    pub music: &'a [Sample],
    pub carrier: &'a [Sample],
}

pub struct MixGains {
    pub mic: f32,
    pub music: f32,
    pub carrier: f32,
    /// If mic RMS < this threshold, carrier is enabled. Set to 0 to always-off.
    pub carrier_gate_rms: f32,
}

impl Default for MixGains {
    fn default() -> Self {
        MixGains { mic: 1.0, music: 0.8, carrier: 0.05, carrier_gate_rms: 0.01 }
    }
}

/// Mix mic + music + (gated) carrier with a soft-knee limiter at +/-1.0.
/// All input slices MUST be the same length; output is a fresh Vec of that length.
pub fn mix(input: MixInput, gains: &MixGains) -> Frame {
    let n = input.mic.len();
    assert_eq!(input.music.len(), n, "music length mismatch");
    assert_eq!(input.carrier.len(), n, "carrier length mismatch");

    let mic_rms = rms(input.mic);
    let carrier_active = mic_rms < gains.carrier_gate_rms;
    let carrier_g = if carrier_active { gains.carrier } else { 0.0 };

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let s = input.mic[i] * gains.mic
              + input.music[i] * gains.music
              + input.carrier[i] * carrier_g;
        out.push(soft_limit(s));
    }
    Frame(out)
}

fn rms(s: &[f32]) -> f32 {
    if s.is_empty() { return 0.0; }
    let sum_sq: f32 = s.iter().map(|x| x * x).sum();
    (sum_sq / s.len() as f32).sqrt()
}

fn soft_limit(s: f32) -> f32 {
    // tanh-based soft clipping; linear up to ~0.7, smooth knee above.
    s.tanh()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_input_yields_silent_output() {
        let mic = [0.0; 480];
        let music = [0.0; 480];
        let carrier = [0.5; 480]; // would activate but should be gated off since mic is "silent"
        let gains = MixGains { carrier_gate_rms: 0.0, ..MixGains::default() };
        // gate_rms = 0 means mic_rms (0.0) is NOT < 0.0, so carrier off
        let out = mix(MixInput { mic: &mic, music: &music, carrier: &carrier }, &gains);
        assert!(out.0.iter().all(|s| s.abs() < 1e-6));
    }

    #[test]
    fn mic_only_passes_through_with_gain() {
        let mic = [0.5; 480];
        let music = [0.0; 480];
        let carrier = [0.0; 480];
        let gains = MixGains { mic: 1.0, music: 0.0, carrier: 0.0, carrier_gate_rms: 0.0 };
        let out = mix(MixInput { mic: &mic, music: &music, carrier: &carrier }, &gains);
        // 0.5.tanh() ≈ 0.4621
        approx::assert_abs_diff_eq!(out.0[0], 0.5_f32.tanh(), epsilon = 1e-5);
    }

    #[test]
    fn carrier_engages_when_mic_below_threshold() {
        let mic = [0.001; 480];   // RMS = 0.001
        let music = [0.0; 480];
        let carrier = [0.4; 480];
        let gains = MixGains { mic: 0.0, music: 0.0, carrier: 1.0, carrier_gate_rms: 0.01 };
        let out = mix(MixInput { mic: &mic, music: &music, carrier: &carrier }, &gains);
        assert!(out.0[0] > 0.3, "carrier should be audible: got {}", out.0[0]);
    }

    #[test]
    fn carrier_disengages_when_mic_above_threshold() {
        let mic = [0.5; 480];     // RMS = 0.5
        let music = [0.0; 480];
        let carrier = [0.4; 480];
        let gains = MixGains { mic: 0.0, music: 0.0, carrier: 1.0, carrier_gate_rms: 0.01 };
        let out = mix(MixInput { mic: &mic, music: &music, carrier: &carrier }, &gains);
        // carrier off → output is mic*0 + music*0 + carrier*0 = 0
        assert!(out.0[0].abs() < 1e-6, "carrier should be gated off: got {}", out.0[0]);
    }

    #[test]
    fn limiter_prevents_clip() {
        let mic = [10.0; 480];   // way over scale
        let music = [10.0; 480];
        let carrier = [0.0; 480];
        let out = mix(MixInput { mic: &mic, music: &music, carrier: &carrier }, &MixGains::default());
        assert!(out.0.iter().all(|s| s.abs() < 1.0), "soft limiter must keep |s|<1");
    }
}
```

- [ ] **Step 2: Run tests, verify all pass**

```bash
cd src-tauri && cargo test --lib audio::mixer && cd ..
```
Expected: 5 passed.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/audio/mixer.rs
git commit -m "feat(audio): mixer with carrier gate and soft limiter, fully unit-tested"
```

---

## Task 3: Mic Capture

**Files:**
- Create: `src-tauri/src/audio/mic_capture.rs`

- [ ] **Step 1: Implement mic_capture.rs**

```rust
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use parking_lot::Mutex;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use super::types::{AudioError, FRAME_SAMPLES, SAMPLE_RATE};

pub struct MicCapture {
    _stream: Stream,
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

        // ~200 ms of ring at 48 kHz mono = 9600 samples — plenty of slack.
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
            SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    push_resampled(data, in_channels, in_sr, &mut producer, &rms_clone);
                },
                |err| eprintln!("mic stream error: {err}"),
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    let f: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    push_resampled(&f, in_channels, in_sr, &mut producer, &rms_clone);
                },
                |err| eprintln!("mic stream error: {err}"),
                None,
            ),
            SampleFormat::U16 => device.build_input_stream(
                &stream_config,
                move |data: &[u16], _| {
                    let f: Vec<f32> = data
                        .iter()
                        .map(|&s| (s as f32 - 32768.0) / 32768.0)
                        .collect();
                    push_resampled(&f, in_channels, in_sr, &mut producer, &rms_clone);
                },
                |err| eprintln!("mic stream error: {err}"),
                None,
            ),
            other => return Err(AudioError::Cpal(format!("unsupported format {:?}", other))),
        }
        .map_err(|e| AudioError::Cpal(e.to_string()))?;

        stream.play().map_err(|e| AudioError::Cpal(e.to_string()))?;
        Ok(Self { _stream: stream, consumer, rms_bits })
    }
}

fn push_resampled(
    data: &[f32],
    channels: u16,
    in_sr: u32,
    producer: &mut Producer<f32>,
    rms_atom: &AtomicU32,
) {
    // Step 1: Downmix to mono.
    let mono: Vec<f32> = if channels == 1 {
        data.to_vec()
    } else {
        data.chunks(channels as usize)
            .map(|c| c.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    // Step 2: Linear resample to SAMPLE_RATE if needed.
    let resampled: Vec<f32> = if in_sr == SAMPLE_RATE {
        mono
    } else {
        linear_resample(&mono, in_sr, SAMPLE_RATE)
    };

    // Step 3: Compute and store RMS for UI.
    let rms = if resampled.is_empty() {
        0.0
    } else {
        let s: f32 = resampled.iter().map(|x| x * x).sum();
        (s / resampled.len() as f32).sqrt()
    };
    rms_atom.store(rms.to_bits(), Ordering::Relaxed);

    // Step 4: Push to ring; drop overflow.
    for s in resampled {
        if producer.push(s).is_err() {
            // ring full — consumer too slow; drop.
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
```

- [ ] **Step 2: Build, run tests**

```bash
cd src-tauri && cargo test --lib audio::mic_capture && cd ..
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/audio/mic_capture.rs
git commit -m "feat(audio): mic capture with downmix + linear resample + ring buffer"
```

---

## Task 4: Music Player (symphonia)

**Files:**
- Create: `src-tauri/src/audio/music_player.rs`

- [ ] **Step 1: Implement music_player.rs**

```rust
use parking_lot::Mutex;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

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
                    return; // remaining slots untouched (caller starts from 0)
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
        // Cap at 8 simultaneous voices; drop oldest if full.
        if self.voices.len() >= 8 {
            self.voices.remove(0);
        }
        self.voices.push(Voice { samples, position: 0, looping, gain });
    }
    pub fn stop_all(&mut self) {
        self.voices.clear();
    }
    pub fn mix_into(&mut self, out: &mut [f32]) {
        for slot in out.iter_mut() { *slot = 0.0; }
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
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AudioError::Decode(e.to_string()))?;
    let mut format: Box<dyn FormatReader> = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| AudioError::Decode("no audio track".into()))?;
    let track_id = track.id;
    let track_sr = track.codec_params.sample_rate.unwrap_or(SAMPLE_RATE);
    let track_channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(1);

    let mut decoder: Box<dyn Decoder> = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioError::Decode(e.to_string()))?;

    let mut mono: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(AudioError::Decode(e.to_string())),
        };
        if packet.track_id() != track_id { continue; }

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
                for ch in 0..channels as usize {
                    sum += b.chan(ch.min(b.spec().channels.count() - 1))[i];
                }
                out.push(sum / channels as f32);
            }
        }
        S16(b) => push_int(b, channels, out, |s| s as f32 / i16::MAX as f32),
        S32(b) => push_int(b, channels, out, |s| s as f32 / i32::MAX as f32),
        U8(b)  => push_int(b, channels, out, |s| (s as f32 - 128.0) / 128.0),
        U16(b) => push_int(b, channels, out, |s| (s as f32 - 32768.0) / 32768.0),
        U32(b) => push_int(b, channels, out, |s| (s as f32 - 2_147_483_648.0) / 2_147_483_648.0),
        S8(b)  => push_int(b, channels, out, |s| s as f32 / i8::MAX as f32),
        F64(b) => {
            let frames = b.frames();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..channels as usize {
                    sum += b.chan(ch.min(b.spec().channels.count() - 1))[i] as f32;
                }
                out.push(sum / channels as f32);
            }
        }
        S24(b) => {
            let frames = b.frames();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..channels as usize {
                    let v = b.chan(ch.min(b.spec().channels.count() - 1))[i].inner();
                    sum += v as f32 / (1 << 23) as f32;
                }
                out.push(sum / channels as f32);
            }
        }
        U24(b) => {
            let frames = b.frames();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..channels as usize {
                    let v = b.chan(ch.min(b.spec().channels.count() - 1))[i].inner();
                    sum += (v as f32 - (1 << 23) as f32) / (1 << 23) as f32;
                }
                out.push(sum / channels as f32);
            }
        }
    }
}

fn push_int<T: Copy>(
    b: &symphonia::core::audio::AudioBuffer<T>,
    channels: u16,
    out: &mut Vec<f32>,
    norm: impl Fn(T) -> f32,
) where T: symphonia::core::sample::Sample {
    let frames = b.frames();
    for i in 0..frames {
        let mut sum = 0.0f32;
        for ch in 0..channels as usize {
            sum += norm(b.chan(ch.min(b.spec().channels.count() - 1))[i]);
        }
        out.push(sum / channels as f32);
    }
}
```

- [ ] **Step 2: Make `linear_resample` reusable**

In `mic_capture.rs`, add a public re-export:

```rust
pub fn linear_resample_pub(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    linear_resample(input, from, to)
}
```

- [ ] **Step 3: Add a smoke test**

In `music_player.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_pull_loops() {
        let mut v = Voice { samples: vec![1.0, 2.0, 3.0], position: 0, looping: true, gain: 1.0 };
        let mut out = [0.0_f32; 5];
        v.pull(&mut out);
        assert_eq!(out, [1.0, 2.0, 3.0, 1.0, 2.0]);
    }

    #[test]
    fn voice_pull_oneshot_finishes() {
        let mut v = Voice { samples: vec![1.0, 2.0], position: 0, looping: false, gain: 1.0 };
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
        for s in &out { approx::assert_abs_diff_eq!(*s, 0.7, epsilon = 1e-5); }
    }
}
```

- [ ] **Step 4: Build and test**

```bash
cd src-tauri && cargo test --lib audio::music_player && cd ..
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/audio/music_player.rs src-tauri/src/audio/mic_capture.rs
git commit -m "feat(audio): music decoder + voice pool + mix-into"
```

---

## Task 5: Carrier (Loadable Voice Sample with Gate)

**Files:**
- Create: `src-tauri/src/audio/carrier.rs`

- [ ] **Step 1: Implement carrier.rs**

```rust
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
    pub fn loaded(&self) -> bool { !self.samples.is_empty() }
    /// Fill `out` with carrier samples, looping. If unloaded, fills with very low pink noise.
    pub fn pull(&mut self, out: &mut [f32]) {
        if self.samples.is_empty() {
            for slot in out.iter_mut() { *slot = 0.0; }
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
```

- [ ] **Step 2: Build and test**

```bash
cd src-tauri && cargo test --lib audio::carrier && cd ..
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/audio/carrier.rs
git commit -m "feat(audio): voice carrier with file loading and silence fallback"
```

---

## Task 6: Virtual Sink (Output to VB-Cable)

**Files:**
- Create: `src-tauri/src/audio/virtual_sink.rs`

- [ ] **Step 1: Implement virtual_sink.rs**

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream};

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
    host.output_devices().ok()?.find(|d| d.name().map(|n| n == name).unwrap_or(false))
}

pub fn find_input_device(host: &Host, name: &str) -> Option<Device> {
    host.input_devices().ok()?.find(|d| d.name().map(|n| n == name).unwrap_or(false))
}

pub fn detect_vb_cable(host: &Host) -> Option<String> {
    list_output_devices(host)
        .into_iter()
        .find(|n| n.contains(VB_CABLE_INPUT_HINT))
}

pub struct VirtualSink {
    _stream: Stream,
}

impl VirtualSink {
    /// Start writing to `device`. `fill` is called from the audio thread for each output buffer.
    pub fn start<F>(device: &Device, mut fill: F) -> Result<Self, AudioError>
    where F: FnMut(&mut [f32]) + Send + 'static {
        let config = device.default_output_config().map_err(|e| AudioError::Cpal(e.to_string()))?;
        let channels = config.channels() as usize;
        let sample_format = config.sample_format();
        let stream_config: cpal::StreamConfig = config.into();

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _| {
                    let frames = data.len() / channels;
                    let mut mono = vec![0.0f32; frames];
                    fill(&mut mono);
                    for (i, frame) in data.chunks_mut(channels).enumerate() {
                        let s = mono[i];
                        for ch in frame.iter_mut() { *ch = s; }
                    }
                },
                |err| eprintln!("output stream error: {err}"),
                None,
            ),
            SampleFormat::I16 => device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _| {
                    let frames = data.len() / channels;
                    let mut mono = vec![0.0f32; frames];
                    fill(&mut mono);
                    for (i, frame) in data.chunks_mut(channels).enumerate() {
                        let s = (mono[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        for ch in frame.iter_mut() { *ch = s; }
                    }
                },
                |err| eprintln!("output stream error: {err}"),
                None,
            ),
            other => return Err(AudioError::Cpal(format!("unsupported output format {:?}", other))),
        }
        .map_err(|e| AudioError::Cpal(e.to_string()))?;

        stream.play().map_err(|e| AudioError::Cpal(e.to_string()))?;
        Ok(Self { _stream: stream })
    }
}
```

- [ ] **Step 2: Build**

```bash
cd src-tauri && cargo check && cd ..
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/audio/virtual_sink.rs
git commit -m "feat(audio): output sink + device enumeration + VB-Cable detection"
```

---

## Task 7: Engine — Wire It All Together

**Files:**
- Create: `src-tauri/src/audio/engine.rs`

- [ ] **Step 1: Implement engine.rs**

```rust
use cpal::traits::HostTrait;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
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
    _mic: MicCapture,
    _sink: VirtualSink,
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
                for s in out_mono.iter_mut() { *s = 0.0; }
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
                MixInput { mic: &mic_buf, music: &music_buf, carrier: &carrier_buf },
                &g,
            );
            out_mono.copy_from_slice(&mixed.0);

            let mut sum_sq = 0.0f32;
            for s in out_mono.iter() { sum_sq += s * s; }
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
```

- [ ] **Step 2: Make `MixGains` Copy**

In `mixer.rs`, derive `Clone, Copy` on `MixGains`:

```rust
#[derive(Debug, Clone, Copy)]
pub struct MixGains { ... }
```

- [ ] **Step 3: Build**

```bash
cd src-tauri && cargo check && cd ..
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/audio/engine.rs src-tauri/src/audio/mixer.rs
git commit -m "feat(audio): audio engine — wires mic → mixer → carrier → sink"
```

---

## Task 8: Library Module

**Files:**
- Create: `src-tauri/src/library.rs`

- [ ] **Step 1: Implement library.rs**

```rust
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Bgm,
    Sfx,
    VoiceLine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryItem {
    pub id: String,
    pub path: PathBuf,
    pub display_name: String,
    pub category: Category,
    pub hotkey: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Library {
    pub items: HashMap<String, LibraryItem>,
}

impl Library {
    pub fn add_file(&mut self, path: PathBuf, category: Category) -> &LibraryItem {
        let id = uuid_like(&path);
        let display_name = path
            .file_stem().and_then(|s| s.to_str())
            .unwrap_or("audio").to_string();
        let item = LibraryItem { id: id.clone(), path, display_name, category, hotkey: None };
        self.items.insert(id.clone(), item);
        self.items.get(&id).unwrap()
    }
    pub fn remove(&mut self, id: &str) { self.items.remove(id); }
    pub fn get(&self, id: &str) -> Option<&LibraryItem> { self.items.get(id) }
    pub fn set_hotkey(&mut self, id: &str, hotkey: Option<String>) {
        if let Some(it) = self.items.get_mut(id) { it.hotkey = hotkey; }
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let json = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, json)
    }
}

fn uuid_like(p: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    p.hash(&mut h);
    std::time::SystemTime::now().hash(&mut h);
    format!("{:016x}", h.finish())
}

pub type SharedLibrary = Arc<RwLock<Library>>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_and_retrieve() {
        let mut lib = Library::default();
        let id = lib.add_file(PathBuf::from("/tmp/song.mp3"), Category::Bgm).id.clone();
        assert_eq!(lib.get(&id).unwrap().display_name, "song");
    }
    #[test]
    fn roundtrip_serialization() {
        let mut lib = Library::default();
        lib.add_file(PathBuf::from("/tmp/x.wav"), Category::Sfx);
        let tmp = std::env::temp_dir().join("wavoic-test-lib.json");
        lib.save(&tmp).unwrap();
        let loaded = Library::load(&tmp);
        assert_eq!(loaded.items.len(), 1);
        std::fs::remove_file(&tmp).ok();
    }
}
```

- [ ] **Step 2: Register module in `main.rs`** — add `mod library;`

- [ ] **Step 3: Build and test**

```bash
cd src-tauri && cargo test --lib library && cd ..
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/library.rs src-tauri/src/main.rs
git commit -m "feat(library): file index with category, hotkey, JSON persistence"
```

---

## Task 9: Hotkey Manager

**Files:**
- Create: `src-tauri/src/hotkeys.rs`

- [ ] **Step 1: Implement hotkeys.rs**

```rust
use anyhow::Result;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub type HotkeyCallback = Arc<dyn Fn() + Send + Sync>;

pub struct HotkeyService {
    manager: GlobalHotKeyManager,
    bindings: Mutex<HashMap<u32, (HotKey, HotkeyCallback)>>,
}

impl HotkeyService {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new()?;
        let svc = Self { manager, bindings: Mutex::new(HashMap::new()) };
        Ok(svc)
    }

    /// Register a hotkey by string like "Ctrl+Shift+F1" or "F1".
    /// Returns the hotkey id.
    pub fn register(&self, accelerator: &str, callback: HotkeyCallback) -> Result<u32> {
        let hk = parse_accelerator(accelerator)?;
        let id = hk.id();
        self.manager.register(hk)?;
        self.bindings.lock().insert(id, (hk, callback));
        Ok(id)
    }

    pub fn unregister(&self, id: u32) -> Result<()> {
        let mut b = self.bindings.lock();
        if let Some((hk, _)) = b.remove(&id) {
            self.manager.unregister(hk)?;
        }
        Ok(())
    }

    /// Call this from the main UI thread loop periodically to dispatch events.
    /// (global-hotkey emits events into its own channel.)
    pub fn poll(&self) {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == global_hotkey::HotKeyState::Pressed {
                if let Some((_, cb)) = self.bindings.lock().get(&event.id) {
                    cb();
                }
            }
        }
    }
}

fn parse_accelerator(s: &str) -> Result<HotKey> {
    let mut mods = Modifiers::empty();
    let mut code: Option<Code> = None;
    for part in s.split('+').map(|p| p.trim()) {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" => mods |= Modifiers::ALT,
            "super" | "meta" | "win" => mods |= Modifiers::META,
            other => code = Some(parse_code(other)?),
        }
    }
    let code = code.ok_or_else(|| anyhow::anyhow!("no key in accelerator '{s}'"))?;
    Ok(HotKey::new(Some(mods), code))
}

fn parse_code(s: &str) -> Result<Code> {
    let upper = s.to_uppercase();
    let code = match upper.as_str() {
        "F1" => Code::F1, "F2" => Code::F2, "F3" => Code::F3, "F4" => Code::F4,
        "F5" => Code::F5, "F6" => Code::F6, "F7" => Code::F7, "F8" => Code::F8,
        "F9" => Code::F9, "F10" => Code::F10, "F11" => Code::F11, "F12" => Code::F12,
        "A" => Code::KeyA, "B" => Code::KeyB, "C" => Code::KeyC, "D" => Code::KeyD,
        "E" => Code::KeyE, "F" => Code::KeyF, "G" => Code::KeyG, "H" => Code::KeyH,
        "I" => Code::KeyI, "J" => Code::KeyJ, "K" => Code::KeyK, "L" => Code::KeyL,
        "M" => Code::KeyM, "N" => Code::KeyN, "O" => Code::KeyO, "P" => Code::KeyP,
        "Q" => Code::KeyQ, "R" => Code::KeyR, "S" => Code::KeyS, "T" => Code::KeyT,
        "U" => Code::KeyU, "V" => Code::KeyV, "W" => Code::KeyW, "X" => Code::KeyX,
        "Y" => Code::KeyY, "Z" => Code::KeyZ,
        "0" => Code::Digit0, "1" => Code::Digit1, "2" => Code::Digit2, "3" => Code::Digit3,
        "4" => Code::Digit4, "5" => Code::Digit5, "6" => Code::Digit6, "7" => Code::Digit7,
        "8" => Code::Digit8, "9" => Code::Digit9,
        "SPACE" => Code::Space,
        "ESC" | "ESCAPE" => Code::Escape,
        other => return Err(anyhow::anyhow!("unknown key '{other}'")),
    };
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_simple() {
        assert!(parse_accelerator("F1").is_ok());
    }
    #[test]
    fn parse_with_modifiers() {
        assert!(parse_accelerator("Ctrl+Shift+A").is_ok());
    }
}
```

- [ ] **Step 2: Register in main.rs** — add `mod hotkeys;`

- [ ] **Step 3: Build and test**

```bash
cd src-tauri && cargo test --lib hotkeys && cd ..
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/hotkeys.rs src-tauri/src/main.rs
git commit -m "feat(hotkeys): global hotkey service with string-accelerator parsing"
```

---

## Task 10: Config and AppState

**Files:**
- Create: `src-tauri/src/config.rs`, `src-tauri/src/state.rs`

- [ ] **Step 1: config.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub carrier_path: Option<PathBuf>,
    pub mic_gain: f32,
    pub music_gain: f32,
    pub carrier_gain: f32,
    pub carrier_gate_rms: f32,
    pub krisp_bypass_enabled: bool,
}

impl Config {
    pub fn with_defaults() -> Self {
        Self {
            input_device: None,
            output_device: None,
            carrier_path: None,
            mic_gain: 1.0,
            music_gain: 0.8,
            carrier_gain: 0.05,
            carrier_gate_rms: 0.01,
            krisp_bypass_enabled: true,
        }
    }
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(Self::with_defaults)
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap())
    }
}

pub fn default_data_dir() -> PathBuf {
    // %APPDATA%\WaVoic on Windows, ~/.config/wavoic on Linux (dev)
    dirs_path().join("WaVoic")
}

fn dirs_path() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}
```

- [ ] **Step 2: state.rs**

```rust
use parking_lot::Mutex;
use std::sync::Arc;

use crate::audio::engine::AudioEngine;
use crate::config::Config;
use crate::hotkeys::HotkeyService;
use crate::library::SharedLibrary;

pub struct AppState {
    pub config: Mutex<Config>,
    pub engine: Mutex<Option<AudioEngine>>,
    pub library: SharedLibrary,
    pub hotkeys: Arc<HotkeyService>,
}

impl AppState {
    pub fn new(config: Config, library: SharedLibrary, hotkeys: Arc<HotkeyService>) -> Self {
        Self {
            config: Mutex::new(config),
            engine: Mutex::new(None),
            library,
            hotkeys,
        }
    }
}
```

- [ ] **Step 3: Register modules in main.rs**

Add `mod config;` and `mod state;`.

- [ ] **Step 4: Add dirs to Cargo.toml? No — we wrote our own.** Build:

```bash
cd src-tauri && cargo check && cd ..
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/state.rs src-tauri/src/main.rs
git commit -m "feat(state): Config, default paths, AppState container"
```

---

## Task 11: Tauri Commands (Frontend ↔ Backend IPC)

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: commands.rs**

```rust
use cpal::traits::HostTrait;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use crate::audio::engine::AudioEngine;
use crate::audio::music_player::decode_to_mono_48k;
use crate::audio::virtual_sink::{detect_vb_cable, list_input_devices, list_output_devices};
use crate::config::{default_data_dir, Config};
use crate::library::{Category, Library, LibraryItem, SharedLibrary};
use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct Devices {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub vb_cable: Option<String>,
}

#[tauri::command]
pub fn list_devices() -> Devices {
    let host = cpal::default_host();
    Devices {
        inputs: list_input_devices(&host),
        outputs: list_output_devices(&host),
        vb_cable: detect_vb_cable(&host),
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().clone()
}

#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> Result<(), String> {
    let path = default_data_dir().join("config.json");
    config.save(&path).map_err(|e| e.to_string())?;
    *state.config.lock() = config;
    Ok(())
}

#[tauri::command]
pub fn start_engine(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = state.config.lock().clone();
    let input = cfg.input_device.ok_or("no input device selected")?;
    let output = cfg.output_device.ok_or("no output device selected")?;
    let engine = AudioEngine::start(&input, &output).map_err(|e| e.to_string())?;
    {
        let mut g = engine.gains.lock();
        g.mic = cfg.mic_gain;
        g.music = cfg.music_gain;
        g.carrier = if cfg.krisp_bypass_enabled { cfg.carrier_gain } else { 0.0 };
        g.carrier_gate_rms = cfg.carrier_gate_rms;
    }
    if let Some(p) = &cfg.carrier_path {
        engine.carrier.lock().load_file(p).map_err(|e| e.to_string())?;
    }
    *state.engine.lock() = Some(engine);
    Ok(())
}

#[tauri::command]
pub fn stop_engine(state: State<'_, AppState>) {
    if let Some(eng) = state.engine.lock().take() {
        eng.stop();
    }
}

#[tauri::command]
pub fn engine_status(state: State<'_, AppState>) -> bool {
    state.engine.lock().is_some()
}

#[tauri::command]
pub fn play_track(state: State<'_, AppState>, id: String, looping: bool) -> Result<(), String> {
    let item = state.library.read().get(&id).cloned()
        .ok_or_else(|| format!("library item '{id}' not found"))?;
    let samples = decode_to_mono_48k(&item.path).map_err(|e| e.to_string())?;
    let engine = state.engine.lock();
    let engine = engine.as_ref().ok_or("engine not running")?;
    engine.player.lock().play(samples, looping, 1.0);
    Ok(())
}

#[tauri::command]
pub fn stop_all(state: State<'_, AppState>) {
    if let Some(eng) = state.engine.lock().as_ref() {
        eng.player.lock().stop_all();
    }
}

#[tauri::command]
pub fn add_library_file(
    state: State<'_, AppState>,
    path: String,
    category: Category,
) -> Result<LibraryItem, String> {
    let mut lib = state.library.write();
    let item = lib.add_file(PathBuf::from(&path), category).clone();
    let lib_path = default_data_dir().join("library.json");
    lib.save(&lib_path).map_err(|e| e.to_string())?;
    Ok(item)
}

#[tauri::command]
pub fn remove_library_file(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut lib = state.library.write();
    lib.remove(&id);
    let lib_path = default_data_dir().join("library.json");
    lib.save(&lib_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_library(state: State<'_, AppState>) -> Vec<LibraryItem> {
    state.library.read().items.values().cloned().collect()
}

#[tauri::command]
pub fn set_hotkey(
    state: State<'_, AppState>,
    id: String,
    accelerator: Option<String>,
) -> Result<(), String> {
    let mut lib = state.library.write();
    lib.set_hotkey(&id, accelerator.clone());
    let lib_path = default_data_dir().join("library.json");
    lib.save(&lib_path).map_err(|e| e.to_string())?;
    // TODO(post-MVP): unregister old, register new live. For now require restart.
    Ok(())
}

#[derive(serde::Serialize)]
pub struct Meters { pub mic_rms: f32, pub output_rms: f32 }

#[tauri::command]
pub fn get_meters(state: State<'_, AppState>) -> Meters {
    if let Some(eng) = state.engine.lock().as_ref() {
        let mic = f32::from_bits(eng.mic_rms_bits.load(std::sync::atomic::Ordering::Relaxed));
        let out = f32::from_bits(eng.output_rms_bits.load(std::sync::atomic::Ordering::Relaxed));
        Meters { mic_rms: mic, output_rms: out }
    } else {
        Meters { mic_rms: 0.0, output_rms: 0.0 }
    }
}
```

- [ ] **Step 2: Wire commands and state in main.rs**

Replace `main()` with:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod config;
mod hotkeys;
mod library;
mod state;

use parking_lot::RwLock;
use std::sync::Arc;

use config::{default_data_dir, Config};
use hotkeys::HotkeyService;
use library::Library;
use state::AppState;

fn main() {
    tracing_subscriber::fmt::init();
    let data_dir = default_data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    let config = Config::load(&data_dir.join("config.json"));
    let library = Arc::new(RwLock::new(Library::load(&data_dir.join("library.json"))));
    let hotkeys = Arc::new(HotkeyService::new().expect("hotkey service init"));
    let app_state = AppState::new(config, library, hotkeys.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .setup(move |app| {
            // Periodically poll hotkeys on a background thread.
            let hk = hotkeys.clone();
            std::thread::spawn(move || loop {
                hk.poll();
                std::thread::sleep(std::time::Duration::from_millis(50));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_devices,
            commands::get_config,
            commands::save_config,
            commands::start_engine,
            commands::stop_engine,
            commands::engine_status,
            commands::play_track,
            commands::stop_all,
            commands::add_library_file,
            commands::remove_library_file,
            commands::list_library,
            commands::set_hotkey,
            commands::get_meters,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Build**

```bash
cd src-tauri && cargo check && cd ..
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(ipc): Tauri commands for devices, engine, library, hotkeys, meters"
```

---

## Task 12: Frontend — Library and Player Bar

**Files:**
- Create: `src/api.ts`, `src/components/library.ts`, `src/components/player-bar.ts`, modify `src/main.ts`, `src/index.html`, `src/styles.css`

- [ ] **Step 1: api.ts**

```ts
import { invoke } from "@tauri-apps/api/core";

export type Category = "bgm" | "sfx" | "voiceline";

export interface LibraryItem {
  id: string;
  path: string;
  display_name: string;
  category: Category;
  hotkey: string | null;
}

export interface Devices { inputs: string[]; outputs: string[]; vb_cable: string | null }
export interface AppConfig {
  input_device: string | null;
  output_device: string | null;
  carrier_path: string | null;
  mic_gain: number;
  music_gain: number;
  carrier_gain: number;
  carrier_gate_rms: number;
  krisp_bypass_enabled: boolean;
}
export interface Meters { mic_rms: number; output_rms: number }

export const api = {
  listDevices: () => invoke<Devices>("list_devices"),
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<void>("save_config", { config }),
  startEngine: () => invoke<void>("start_engine"),
  stopEngine: () => invoke<void>("stop_engine"),
  engineStatus: () => invoke<boolean>("engine_status"),
  playTrack: (id: string, looping: boolean) => invoke<void>("play_track", { id, looping }),
  stopAll: () => invoke<void>("stop_all"),
  addLibraryFile: (path: string, category: Category) =>
    invoke<LibraryItem>("add_library_file", { path, category }),
  removeLibraryFile: (id: string) => invoke<void>("remove_library_file", { id }),
  listLibrary: () => invoke<LibraryItem[]>("list_library"),
  setHotkey: (id: string, accelerator: string | null) =>
    invoke<void>("set_hotkey", { id, accelerator }),
  getMeters: () => invoke<Meters>("get_meters"),
};
```

- [ ] **Step 2: components/library.ts**

```ts
import { api, Category, LibraryItem } from "../api";
import { open } from "@tauri-apps/plugin-dialog";

export class LibraryView {
  el = document.createElement("div");
  items: LibraryItem[] = [];

  constructor() {
    this.el.className = "library";
    this.render();
  }

  async refresh() {
    this.items = await api.listLibrary();
    this.render();
  }

  async addFile() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Audio", extensions: ["mp3", "wav", "flac", "ogg"] }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    for (const p of paths) {
      await api.addLibraryFile(p, "sfx");
    }
    await this.refresh();
  }

  render() {
    this.el.innerHTML = `
      <div class="library-toolbar">
        <button id="addBtn">+ Add Audio</button>
      </div>
      <table class="library-table">
        <thead><tr><th>Name</th><th>Category</th><th>Hotkey</th><th></th></tr></thead>
        <tbody>${this.items.map(this.row).join("")}</tbody>
      </table>
    `;
    this.el.querySelector<HTMLButtonElement>("#addBtn")!.onclick = () => this.addFile();
    this.el.querySelectorAll<HTMLButtonElement>(".play-btn").forEach(btn => {
      btn.onclick = () => api.playTrack(btn.dataset.id!, false);
    });
    this.el.querySelectorAll<HTMLButtonElement>(".loop-btn").forEach(btn => {
      btn.onclick = () => api.playTrack(btn.dataset.id!, true);
    });
    this.el.querySelectorAll<HTMLButtonElement>(".del-btn").forEach(btn => {
      btn.onclick = async () => { await api.removeLibraryFile(btn.dataset.id!); await this.refresh(); };
    });
  }

  row = (it: LibraryItem) => `
    <tr>
      <td>${escape(it.display_name)}</td>
      <td><select data-id="${it.id}" class="cat-sel">
        ${(["bgm","sfx","voiceline"] as Category[]).map(c =>
          `<option value="${c}"${c===it.category?" selected":""}>${c}</option>`).join("")}
      </select></td>
      <td>${it.hotkey ?? "—"}</td>
      <td>
        <button class="play-btn" data-id="${it.id}">▶</button>
        <button class="loop-btn" data-id="${it.id}">⟳</button>
        <button class="del-btn" data-id="${it.id}">✕</button>
      </td>
    </tr>`;
}

function escape(s: string) {
  return s.replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}
```

- [ ] **Step 3: components/player-bar.ts**

```ts
import { api, AppConfig } from "../api";

export class PlayerBar {
  el = document.createElement("div");
  running = false;
  config: AppConfig | null = null;

  constructor() {
    this.el.className = "player-bar";
    this.render();
    this.startMeterLoop();
  }

  async init() {
    this.config = await api.getConfig();
    this.running = await api.engineStatus();
    this.render();
  }

  async toggle() {
    if (this.running) { await api.stopEngine(); }
    else { try { await api.startEngine(); } catch (e) { alert(`Start failed: ${e}`); } }
    this.running = await api.engineStatus();
    this.render();
  }

  async startMeterLoop() {
    while (true) {
      try {
        const m = await api.getMeters();
        const mic = this.el.querySelector<HTMLDivElement>(".meter-mic-fill");
        const out = this.el.querySelector<HTMLDivElement>(".meter-out-fill");
        if (mic) mic.style.width = `${Math.min(100, m.mic_rms * 300)}%`;
        if (out) out.style.width = `${Math.min(100, m.output_rms * 300)}%`;
      } catch {}
      await new Promise(r => setTimeout(r, 100));
    }
  }

  render() {
    this.el.innerHTML = `
      <button id="toggleBtn">${this.running ? "■ Stop" : "▶ Start"}</button>
      <button id="stopAllBtn">⏹ Stop all sounds</button>
      <div class="meter"><div>Mic</div><div class="meter-bar"><div class="meter-mic-fill"></div></div></div>
      <div class="meter"><div>Out</div><div class="meter-bar"><div class="meter-out-fill"></div></div></div>
    `;
    this.el.querySelector<HTMLButtonElement>("#toggleBtn")!.onclick = () => this.toggle();
    this.el.querySelector<HTMLButtonElement>("#stopAllBtn")!.onclick = () => api.stopAll();
  }
}
```

- [ ] **Step 4: Rewrite main.ts to compose**

```ts
import { LibraryView } from "./components/library";
import { PlayerBar } from "./components/player-bar";
import { SettingsView } from "./components/settings";

const root = document.querySelector<HTMLDivElement>("#app")!;
root.innerHTML = `
  <header class="app-header">
    <h1>WaVoic</h1>
    <nav>
      <button data-tab="library">Library</button>
      <button data-tab="settings">Settings</button>
    </nav>
  </header>
  <main id="main"></main>
  <footer id="player"></footer>
`;

const player = new PlayerBar();
document.querySelector("#player")!.appendChild(player.el);
player.init();

const library = new LibraryView();
const settings = new SettingsView(() => player.init());
library.refresh();

const main = document.querySelector<HTMLElement>("#main")!;
function switchTab(tab: string) {
  main.innerHTML = "";
  if (tab === "library") main.appendChild(library.el);
  else if (tab === "settings") main.appendChild(settings.el);
}
document.querySelectorAll<HTMLButtonElement>("nav button").forEach(b => {
  b.onclick = () => switchTab(b.dataset.tab!);
});
switchTab("library");
```

- [ ] **Step 5: styles.css**

```css
* { box-sizing: border-box; }
body { margin: 0; font: 14px system-ui, sans-serif; background: #1a1c20; color: #eee; }
.app-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 20px; background: #14151a; border-bottom: 1px solid #2a2d33; }
.app-header h1 { margin: 0; font-size: 18px; font-weight: 600; }
.app-header nav button { background: transparent; border: 1px solid #3a3d44; color: #ccc; padding: 6px 14px; margin-left: 8px; border-radius: 4px; cursor: pointer; }
.app-header nav button:hover { background: #2a2d33; }
#main { padding: 20px; min-height: calc(100vh - 130px); }
.library-toolbar { margin-bottom: 12px; }
.library-toolbar button { padding: 8px 16px; background: #4a6cf7; border: none; color: white; border-radius: 4px; cursor: pointer; }
.library-table { width: 100%; border-collapse: collapse; }
.library-table th, .library-table td { padding: 8px; text-align: left; border-bottom: 1px solid #2a2d33; }
.library-table button { background: #2a2d33; border: 1px solid #3a3d44; color: #eee; padding: 4px 10px; margin-right: 4px; cursor: pointer; border-radius: 3px; }
.player-bar { display: flex; align-items: center; gap: 16px; padding: 12px 20px; background: #14151a; border-top: 1px solid #2a2d33; }
.player-bar button { padding: 6px 12px; background: #2a2d33; border: 1px solid #3a3d44; color: #eee; border-radius: 4px; cursor: pointer; }
.meter { display: flex; align-items: center; gap: 8px; }
.meter-bar { width: 120px; height: 8px; background: #2a2d33; border-radius: 4px; overflow: hidden; }
.meter-mic-fill, .meter-out-fill { height: 100%; background: #4a6cf7; width: 0%; transition: width 0.1s; }
.settings { display: flex; flex-direction: column; gap: 12px; max-width: 600px; }
.settings label { display: flex; flex-direction: column; gap: 4px; }
.settings select, .settings input { padding: 6px; background: #14151a; border: 1px solid #3a3d44; color: #eee; border-radius: 4px; }
```

- [ ] **Step 6: Commit**

```bash
git add src/api.ts src/components/library.ts src/components/player-bar.ts src/main.ts src/styles.css src/index.html
git commit -m "feat(ui): library, player bar, layout shell"
```

---

## Task 13: Settings View

**Files:**
- Create: `src/components/settings.ts`

- [ ] **Step 1: settings.ts**

```ts
import { api, AppConfig, Devices } from "../api";
import { open } from "@tauri-apps/plugin-dialog";

export class SettingsView {
  el = document.createElement("div");
  config: AppConfig | null = null;
  devices: Devices | null = null;
  onChange: () => void;

  constructor(onChange: () => void) {
    this.el.className = "settings";
    this.onChange = onChange;
    this.load();
  }

  async load() {
    this.devices = await api.listDevices();
    this.config = await api.getConfig();
    this.render();
  }

  async save() {
    if (!this.config) return;
    await api.saveConfig(this.config);
    this.onChange();
  }

  async pickCarrier() {
    const sel = await open({
      multiple: false,
      filters: [{ name: "Audio", extensions: ["wav", "mp3", "flac", "ogg"] }],
    });
    if (sel && typeof sel === "string" && this.config) {
      this.config.carrier_path = sel;
      await this.save();
      this.render();
    }
  }

  render() {
    if (!this.config || !this.devices) { this.el.innerHTML = "Loading…"; return; }
    const c = this.config;
    const d = this.devices;
    this.el.innerHTML = `
      <h2>Settings</h2>

      ${d.vb_cable ? "" : `<div style="background:#5a2a2a;padding:8px;border-radius:4px;margin-bottom:12px;">
        ⚠ VB-Cable not detected. Install from <a href="https://vb-audio.com/Cable/" target="_blank" style="color:#9af">vb-audio.com/Cable</a> and restart Windows.
      </div>`}

      <label>Input Device (your real microphone)
        <select id="inputSel">
          <option value="">—</option>
          ${d.inputs.map(n => `<option value="${n}"${n===c.input_device?" selected":""}>${n}</option>`).join("")}
        </select>
      </label>

      <label>Output Device (must be VB-Cable Input)
        <select id="outputSel">
          <option value="">—</option>
          ${d.outputs.map(n => `<option value="${n}"${n===c.output_device?" selected":""}>${n}</option>`).join("")}
        </select>
      </label>

      <label><input type="checkbox" id="krispBox"${c.krisp_bypass_enabled?" checked":""}/> Krisp bypass (recommended)</label>

      <label>Carrier sample file (optional, used when you're silent)
        <div style="display:flex;gap:8px;">
          <input type="text" value="${c.carrier_path ?? ""}" readonly style="flex:1"/>
          <button id="pickCarrierBtn">Browse…</button>
        </div>
      </label>

      <label>Mic gain: <span id="micVal">${c.mic_gain.toFixed(2)}</span>
        <input type="range" min="0" max="2" step="0.05" value="${c.mic_gain}" id="micGain"/></label>
      <label>Music gain: <span id="musVal">${c.music_gain.toFixed(2)}</span>
        <input type="range" min="0" max="2" step="0.05" value="${c.music_gain}" id="musicGain"/></label>
      <label>Carrier gain: <span id="carVal">${c.carrier_gain.toFixed(2)}</span>
        <input type="range" min="0" max="0.5" step="0.005" value="${c.carrier_gain}" id="carrierGain"/></label>
      <label>Carrier gate (RMS threshold): <span id="gateVal">${c.carrier_gate_rms.toFixed(3)}</span>
        <input type="range" min="0" max="0.1" step="0.001" value="${c.carrier_gate_rms}" id="gateRms"/></label>

      <div style="display:flex;gap:8px;margin-top:12px;">
        <button id="applyBtn">Apply (restart engine)</button>
      </div>
    `;

    const bindSel = (id: string, key: keyof AppConfig) => {
      this.el.querySelector<HTMLSelectElement>(`#${id}`)!.onchange = (e) => {
        (this.config as any)[key] = (e.target as HTMLSelectElement).value || null;
        this.save();
      };
    };
    bindSel("inputSel", "input_device");
    bindSel("outputSel", "output_device");

    this.el.querySelector<HTMLInputElement>("#krispBox")!.onchange = (e) => {
      this.config!.krisp_bypass_enabled = (e.target as HTMLInputElement).checked;
      this.save();
    };

    this.el.querySelector<HTMLButtonElement>("#pickCarrierBtn")!.onclick = () => this.pickCarrier();

    const bindRange = (id: string, key: keyof AppConfig, lbl: string) => {
      const el = this.el.querySelector<HTMLInputElement>(`#${id}`)!;
      el.oninput = () => {
        const v = parseFloat(el.value);
        (this.config as any)[key] = v;
        this.el.querySelector<HTMLSpanElement>(`#${lbl}`)!.innerText = v.toFixed(key.includes("gate") ? 3 : 2);
      };
      el.onchange = () => this.save();
    };
    bindRange("micGain", "mic_gain", "micVal");
    bindRange("musicGain", "music_gain", "musVal");
    bindRange("carrierGain", "carrier_gain", "carVal");
    bindRange("gateRms", "carrier_gate_rms", "gateVal");

    this.el.querySelector<HTMLButtonElement>("#applyBtn")!.onclick = async () => {
      await api.stopEngine();
      try { await api.startEngine(); } catch (e) { alert(`Start failed: ${e}`); }
      this.onChange();
    };
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/settings.ts
git commit -m "feat(ui): settings view with devices, gains, carrier picker, Krisp toggle"
```

---

## Task 14: Tauri Capabilities (Permissions)

**Files:**
- Create: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Create capabilities file**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "default capability set",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:default",
    "fs:allow-read-file",
    "fs:allow-write-file",
    "fs:allow-read-dir",
    "fs:allow-mkdir"
  ]
}
```

- [ ] **Step 2: Add capabilities reference in tauri.conf.json**

Make sure `tauri.conf.json` has `"app": { "windows": [{ "label": "main", ... }] }` — update if missing.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/capabilities/
git commit -m "chore(tauri): capability set for default window"
```

---

## Task 15: End-to-End Smoke + Manual Test Plan

This task is run on Windows only. There are no automated checks here — the goal is to validate the audio path works against real Valorant.

- [ ] **Step 1: Sync code to Windows machine (or build there directly)**

If developing on the Linux remote, `git clone` / `rsync` to the Windows box.

- [ ] **Step 2: Install prerequisites on Windows**

- Rust via rustup
- Node.js ≥ 20
- VB-Cable from vb-audio.com
- Visual Studio Build Tools (C++)
- Reboot

- [ ] **Step 3: Build and run dev**

```powershell
npm install
npm run tauri dev
```

- [ ] **Step 4: Configure WaVoic**

1. Settings → pick your real mic as Input.
2. Settings → pick "CABLE Input (VB-Audio Virtual Cable)" as Output.
3. Library → drop in a few MP3s.
4. Player bar → click Start. Verify Output meter shows movement when you talk.

- [ ] **Step 5: Configure Valorant**

Valorant → Settings → Audio → Voice Chat → Input Device = "CABLE Output".

- [ ] **Step 6: Test with a friend**

1. Join voice chat with a friend.
2. Talk normally — confirm friend hears you.
3. Click ▶ on a music track in WaVoic.
4. **Verify friend hears continuous music without choppiness.**
5. Disable Krisp bypass in Settings → restart engine → verify choppiness returns (negative control).
6. Re-enable Krisp bypass → restart → verify smoothness restored.

- [ ] **Step 7: Tag release v0.1**

```bash
git tag -a v0.1.0 -m "WaVoic v0.1: MVP — Valorant voice-chat music with Krisp bypass"
```

---

## Self-Review Notes

- **Spec coverage**: All Section 5 components in the spec map to tasks (mic_capture=T3, music_player=T4, mixer=T2, virtual_sink=T6, carrier=T5, engine=T7, library=T8, hotkeys=T9). Setup wizard from spec §5.3 deferred to v0.2 — replaced by the Settings VB-Cable detection banner in T13. Acceptable for MVP.
- **Type consistency**: `Category` enum is shared between Rust (`#[serde(rename_all = "lowercase")]`) and TS (`"bgm" | "sfx" | "voiceline"`) — matches.
- **Placeholder check**: One `TODO(post-MVP)` left intentionally in `set_hotkey` (live re-register on rename). Documented as v0.2 work.
- **Risk**: Engine starts on user click rather than auto. Acceptable — gives the user control over when the virtual mic is active.
