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
        let svc = Self {
            manager,
            bindings: Mutex::new(HashMap::new()),
        };
        Ok(svc)
    }

    /// Register a hotkey by string like "Ctrl+Shift+F1" or "F1".
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

    /// Call this from a main-loop thread to dispatch hotkey events.
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
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
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
