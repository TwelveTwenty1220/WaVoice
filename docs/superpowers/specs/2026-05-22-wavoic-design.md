# WaVoic — Design Spec

**Date:** 2026-05-22
**Status:** Approved (user delegated subsequent decisions)
**Target Platform:** Windows 10/11 x64

## 1. Goal

A Windows desktop app that plays user-curated music and sound effects into Valorant's
voice chat so teammates can hear them — for morale moments (round-win BGM, ace SFX),
team jokes, and quick reactions.

The differentiator versus existing tools (Soundpad, raw VB-Cable + media player) is
**reliable playback under Valorant's built-in Krisp AI noise suppression**, which
otherwise classifies music as noise and renders it choppy/intermittent for listeners.

## 2. Non-Goals (Explicit YAGNI)

- Not a Discord/Teams soundboard. Optimized for Valorant's audio pipeline specifically.
- No screen capture / OCR / game-state auto-trigger in v1 (Riot Vanguard friction;
  game-state hotkeys are a planned v2 enhancement).
- No streaming-source capture (Spotify/Netease) in v1. Local files only.
- No multi-instance audio (one music track at a time).
- No mobile, no Mac, no Linux runtime (Valorant is Windows-only).
- Krisp-bypass via custom audio drivers — relies on VB-Cable as the virtual device.

## 3. The Krisp Problem (Why This Exists)

Valorant ships with Krisp's AI noise suppression on outgoing voice. Krisp uses a deep
learning model trained to **keep human voice and discard everything else**. Music
played through a virtual mic gets classified as "not voice" and aggressively gated,
which produces the choppy artifact users see with Soundpad-style approaches.

### Mitigation Strategy: Voice Carrier Mixing

The model treats audio as "voice" when it detects formants, harmonics, and the
spectral profile of human speech. So instead of fighting the classifier, we **feed it
something it wants**: continuously mix the user's real microphone signal underneath
the music output.

- When the user is talking: Krisp sees voice → keeps the whole mix (voice + music) flowing.
- When the user is silent: their mic still carries breath/ambient room noise above
  Krisp's gate threshold for typical configurations. If not sufficient, we add a very
  low-level (~-40 dBFS) pre-recorded vocal sample looped underneath as backup.

This is sufficient to keep Krisp's VAD in "voice present" state and prevent music
gating. We do **not** attempt to defeat Krisp's classifier itself — we satisfy it.

### Why other approaches were rejected

| Approach | Reason rejected |
|---|---|
| Custom kernel-level virtual audio driver | Requires Microsoft WHQL signing (~$500/yr + delays). Out of scope for personal project. |
| Spectral reshaping music into voice band (300-3400 Hz EQ) | Severely degrades music quality. Krisp's modern model is robust to simple EQ tricks anyway. |
| Disable Krisp in Valorant settings | Krisp on outgoing voice is not user-toggleable as of latest patches; only incoming-side voice processing is controllable. |
| Riot Games API for game state | High latency, requires API key, doesn't surface kill/round events with usable timeliness. |

## 4. System Architecture

```
┌──────────────────────────────────────────┐
│  WaVoic.exe (Tauri bundle)               │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │  Frontend (Web: HTML/TS)           │  │
│  │   - Audio library browser          │  │
│  │   - Hotkey binding UI              │  │
│  │   - Play / stop / volume controls  │  │
│  │   - Realtime level meter           │  │
│  │   - Settings (devices, carrier)    │  │
│  └──────────────┬─────────────────────┘  │
│                 │ Tauri IPC (commands +   │
│                 │ events)                 │
│  ┌──────────────▼─────────────────────┐  │
│  │  Backend (Rust)                    │  │
│  │                                    │  │
│  │  ┌─────────────┐  ┌──────────────┐ │  │
│  │  │ MicCapture  │  │ MusicPlayer  │ │  │
│  │  │ (cpal in)   │  │ (symphonia)  │ │  │
│  │  └──────┬──────┘  └──────┬───────┘ │  │
│  │         └────────┬───────┘         │  │
│  │           ┌──────▼────────┐        │  │
│  │           │   Mixer       │        │  │
│  │           │ + CarrierGen  │        │  │
│  │           │ + Limiter     │        │  │
│  │           └──────┬────────┘        │  │
│  │           ┌──────▼────────┐        │  │
│  │           │ VirtualSink   │        │  │
│  │           │ (cpal out →   │        │  │
│  │           │  VB-Cable In) │        │  │
│  │           └──────┬────────┘        │  │
│  │                                    │  │
│  │  HotkeyManager  AudioLibrary       │  │
│  │  (global-hotkey)  (file DB)        │  │
│  └────────────────────────────────────┘  │
└─────────────────────┬────────────────────┘
                      │ PCM stream
                      ▼
            ┌─────────────────────┐
            │ VB-Cable (external) │  ◄─ user installs once
            └──────────┬──────────┘
                       │ exposed as "CABLE Output"
                       ▼ Valorant reads as mic
            ┌─────────────────────┐
            │ Valorant + Krisp    │
            └──────────┬──────────┘
                       ▼
                 Teammates' ears
```

## 5. Components

### 5.1 Backend — Rust

All modules live under `src-tauri/src/audio/`.

#### `mic_capture.rs` — MicCapture
- Owns a `cpal::Stream` reading from the user's chosen input device (their real headset mic).
- Format: 48 kHz, mono, f32 samples. Resamples non-48 kHz inputs.
- Pushes ~10 ms frames onto a lock-free SPSC ring (e.g., `rtrb`) for the mixer to consume.
- Exposes RMS level for the frontend meter via a separate atomic.

#### `music_player.rs` — MusicPlayer
- Decodes the currently-loaded track with `symphonia` (mp3/wav/flac/ogg).
- Maintains playback state: `Idle | Playing { sample_pos } | Paused`.
- Resamples to 48 kHz mono. Stereo sources are downmixed.
- Each "play track" command can be a one-shot (SFX) or looping (background music).
- Multiple concurrent triggered SFX are queued and overlapped via an internal voice
  pool (8 voices max) — the mixer sums them.

#### `mixer.rs` — Mixer
- Single audio thread, fixed 10 ms tick driven by the output stream.
- Each tick:
  1. Pull `mic_frame` from MicCapture ring (zero-fill if underrun).
  2. Pull `music_frame` from MusicPlayer (sum of active voices).
  3. If `mic_rms < carrier_threshold` and a `carrier_sample` is loaded, mix in
     `carrier_sample` at user-configured low level (default -40 dBFS).
  4. Compute output = `mic * mic_gain + music * music_gain + carrier * carrier_gain`.
  5. Soft-knee limiter to avoid clipping near full scale.
  6. Hand `output_frame` to VirtualSink.
- All gains are atomics, changeable from the UI thread without locks.

#### `virtual_sink.rs` — VirtualSink
- Owns a `cpal::Stream` writing to the user-chosen output device (default: "CABLE Input (VB-Audio Virtual Cable)").
- Validates at startup that VB-Cable is installed. If not, surfaces a setup-wizard
  event to the frontend.

#### `hotkey_manager.rs` — HotkeyManager
- Wraps the `global-hotkey` crate.
- Per-clip and per-action bindings:
  - **Per-clip**: play a specific track on press.
  - **Action keys**: `stop_all`, `panic_mute`, `volume_up`, `volume_down`, `cycle_track`.
- Persists bindings in `config.json`.

#### `audio_library.rs` — AudioLibrary
- Indexes a user-chosen root folder (`~/AppData/Roaming/WaVoic/library/` by default).
- File metadata: path, duration, optional tags, optional bound hotkey, category
  (BGM / SFX / Voice-line).
- Caches duration by scanning headers; doesn't decode full files at import time.
- Persists to `library.json`.

### 5.2 Frontend — Web (Tauri WebView)

Stack: TypeScript + vanilla DOM (no heavy framework — keeps bundle <1 MB and load <100 ms).

Pages:
- **Library**: drag-and-drop import; list with categories, search, hotkey badges.
- **Player bar** (always visible): currently playing track, transport controls, master volumes (mic / music / carrier).
- **Hotkeys**: capture + reassign; conflict detection.
- **Settings**: input device, output device (VB-Cable), carrier sample picker,
  Krisp-bypass mode toggle.

All UI state is held in the backend and pushed to the frontend via Tauri events;
the frontend is a thin renderer.

### 5.3 Setup / Onboarding

First-run wizard:
1. Detect VB-Cable. If missing, open VB-Audio's download page and explain steps.
2. Confirm input device (live VU meter to verify mic).
3. Confirm output device — must be "CABLE Input"; warn if user picks a real output.
4. Test loop: prompts the user to talk + plays a 5-second music sample mixed into
   the output. The frontend shows the same waveform that's being sent to Valorant
   so the user can sanity-check.
5. Brief instructions for Valorant settings: set microphone to "CABLE Output".

## 6. Data Flow (Per 10 ms Tick)

```
T+0ms:   cpal_input_callback fires → mic_frame_buf.push(samples)
T+0ms:   cpal_output_callback fires (driven by VB-Cable's sample clock)
         ├─ pull mic_frame  (≤10ms old)
         ├─ pull music_frame from MusicPlayer voices
         ├─ carrier_gate = (mic_rms < threshold) ? carrier_gain : 0
         ├─ mix = mic*g_mic + music*g_music + carrier_sample*carrier_gate
         ├─ limit(mix)
         └─ write to cpal output buffer → VB-Cable → Valorant
```

Target end-to-end mic-to-virtual-output latency: **<30 ms** (cpal default buffer
sizes on WASAPI shared mode achieve 10-20 ms in practice).

## 7. Error Handling

| Failure mode | Detection | Behavior |
|---|---|---|
| VB-Cable not installed | `cpal::devices()` lookup at startup | Block app use; show installer link |
| Selected input device disappears | cpal stream error event | Pause output; surface toast; re-prompt device selection |
| Music decode error | symphonia error | Skip track, log, surface inline error in library row |
| Output buffer underrun | cpal callback returns silence | Insert silence; log; if >5 underruns/min, surface warning |
| Hotkey already registered by another app | `global-hotkey` returns conflict | UI shows red badge; ask user to pick a different binding |
| Sample-rate mismatch between devices | Detected at stream init | Auto-resample using `rubato` |

We deliberately do **not** handle: VB-Cable being uninstalled mid-session
(rare; user can restart), Windows audio service restarting (cpal usually recovers
on its own), or Valorant not detecting the virtual mic (user-side config issue, not
ours).

## 8. Testing Strategy

### Unit tests (Rust)
- Mixer math: golden-file tests for known inputs producing known sums.
- Limiter: verifies no clip past ±1.0 across a sweep of input levels.
- Carrier gate: state-machine tests for threshold hysteresis.
- AudioLibrary: import / persist / reload roundtrip.

### Integration tests (Rust)
- End-to-end with `cpal`'s null device backend: feed synthetic input + music,
  capture output, assert RMS and spectral expectations.

### Manual test plan (must run on Windows with Valorant)
1. Solo lobby + a second account / friend on a different machine.
2. Play known-difficult content: continuous instrumental music (where Krisp gating
   has historically been worst).
3. Friend confirms continuous playback without choppiness for 60 seconds.
4. Repeat with carrier disabled — should reproduce the choppy artifact (negative
   control).
5. User talks over music — verify both are audible to friend.
6. Hotkey-triggered SFX during gunfight — verify ≤200 ms perceived latency.

## 9. Project Layout

```
WaVoic/
├── docs/superpowers/specs/
│   └── 2026-05-22-wavoic-design.md     (this file)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                     Tauri app entry + IPC commands
│   │   ├── audio/
│   │   │   ├── mod.rs
│   │   │   ├── mic_capture.rs
│   │   │   ├── music_player.rs
│   │   │   ├── mixer.rs
│   │   │   ├── virtual_sink.rs
│   │   │   └── carrier.rs              voice-carrier generator + gate
│   │   ├── hotkey_manager.rs
│   │   ├── audio_library.rs
│   │   └── config.rs                   load/save config.json
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                                 (frontend)
│   ├── index.html
│   ├── main.ts
│   ├── components/
│   │   ├── library.ts
│   │   ├── player-bar.ts
│   │   ├── hotkeys.ts
│   │   └── settings.ts
│   └── styles.css
├── package.json
└── README.md
```

## 10. Key Dependencies (Rust)

| Crate | Purpose |
|---|---|
| `tauri` (v2) | App shell, IPC, window |
| `cpal` | Cross-platform audio I/O (Windows: WASAPI backend) |
| `symphonia` | Audio file decoding (mp3/wav/flac/ogg) |
| `rubato` | Sample-rate conversion |
| `rtrb` | Lock-free SPSC ring buffer |
| `global-hotkey` | System-wide hotkey registration |
| `serde` / `serde_json` | Config + library persistence |
| `parking_lot` | Faster mutexes for UI state |

## 11. MVP vs Future

**MVP (v0.1, this iteration):**
- Single-track music playback + multiple SFX overlap
- Mic + music + (optional) prerecorded carrier mixing
- Local file library with drag-and-drop import
- Global hotkeys
- VB-Cable detection + setup wizard
- First-run config flow

**v0.2 candidates:**
- Per-track auto-fade / ducking when user talks
- Playlist / shuffle for background music
- WASAPI loopback capture as alternate source (route any system audio)
- TTS-generated carrier (better Krisp robustness when user is silent)
- Game-state detection via screen reader / log files (Valorant client logs to
  `%LOCALAPPDATA%\VALORANT\Saved\Logs`; round transitions are loggable)

## 12. Open Questions

None blocking MVP. Dev environment is Linux remote (PyCharm SSH); Windows-specific
audio behavior must be validated on a Windows machine before each release. We will
sync the source tree to a Windows box and `cargo run` from there for any
audio-path changes.
