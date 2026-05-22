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
        g.carrier = if cfg.krisp_bypass_enabled {
            cfg.carrier_gain
        } else {
            0.0
        };
        g.carrier_gate_rms = cfg.carrier_gate_rms;
    }
    if let Some(p) = &cfg.carrier_path {
        engine
            .carrier
            .lock()
            .load_file(p)
            .map_err(|e| e.to_string())?;
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
    let item = state
        .library
        .read()
        .get(&id)
        .cloned()
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
    Ok(())
}

#[derive(serde::Serialize)]
pub struct Meters {
    pub mic_rms: f32,
    pub output_rms: f32,
}

#[tauri::command]
pub fn get_meters(state: State<'_, AppState>) -> Meters {
    if let Some(eng) = state.engine.lock().as_ref() {
        let mic =
            f32::from_bits(eng.mic_rms_bits.load(std::sync::atomic::Ordering::Relaxed));
        let out =
            f32::from_bits(eng.output_rms_bits.load(std::sync::atomic::Ordering::Relaxed));
        Meters {
            mic_rms: mic,
            output_rms: out,
        }
    } else {
        Meters {
            mic_rms: 0.0,
            output_rms: 0.0,
        }
    }
}
