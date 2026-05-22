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
