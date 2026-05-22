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
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("audio")
            .to_string();
        let item = LibraryItem {
            id: id.clone(),
            path,
            display_name,
            category,
            hotkey: None,
        };
        self.items.insert(id.clone(), item);
        self.items.get(&id).unwrap()
    }
    pub fn remove(&mut self, id: &str) {
        self.items.remove(id);
    }
    pub fn get(&self, id: &str) -> Option<&LibraryItem> {
        self.items.get(id)
    }
    pub fn set_hotkey(&mut self, id: &str, hotkey: Option<String>) {
        if let Some(it) = self.items.get_mut(id) {
            it.hotkey = hotkey;
        }
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
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
        let id = lib
            .add_file(PathBuf::from("/tmp/song.mp3"), Category::Bgm)
            .id
            .clone();
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
