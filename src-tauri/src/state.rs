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
