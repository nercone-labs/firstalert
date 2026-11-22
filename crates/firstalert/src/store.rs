//! The files of the core: the settings and the history, each written whole and atomically (to
//! a temporary file that then replaces the old one), so that a crash never leaves half a file.

use crate::alert::AlertLevel;
use crate::event::{EarthquakeEvent, Observation};
use crate::settings::Settings;
use crate::tsunami::{SpecialEntry, TsunamiState};
use jma::Intensity;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// What has been notified under a key, so that an update alerts again only when it is more
/// severe, also after a restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notified {
    pub level: AlertLevel,
    pub intensity: Option<Intensity>,
    /// What the notification showed (a serial number, a time of issue), to skip repeats.
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredEvent {
    pub event: EarthquakeEvent,
    pub observations: Vec<Observation>,
}

/// The history the core keeps across restarts.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct History {
    pub version: u32,
    pub earthquakes: Vec<StoredEvent>,
    pub tsunami: Option<TsunamiState>,
    pub special: Vec<SpecialEntry>,
    pub notified: BTreeMap<String, Notified>,
    /// The ids of the P2P Quake messages already handled.
    pub seen: Vec<String>,
}

impl History {
    pub const VERSION: u32 = 1;
}

/// The directory of the files; without one, nothing is kept.
#[derive(Debug, Clone, PartialEq)]
pub struct Store {
    pub directory: Option<PathBuf>,
}

impl Store {
    pub const SETTINGS: &'static str = "settings.json";
    pub const HISTORY: &'static str = "history.json";

    /// Reads `name`; a missing file is `Ok(None)`. A file that cannot be read as JSON is kept
    /// aside (`.broken`) so that it is neither lost nor read again, and is an error.
    fn read<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, String> {
        let Some(directory) = &self.directory else { return Ok(None) };
        let path = directory.join(name);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        };
        match serde_json::from_slice(&bytes) {
            Ok(value) => Ok(Some(value)),
            Err(error) => {
                let aside = directory.join(format!("{name}.broken"));
                let _ = fs::rename(&path, &aside);
                Err(format!("{} is damaged ({error}); kept as {}", path.display(), aside.display()))
            }
        }
    }

    fn write<T: Serialize>(&self, name: &str, value: &T) -> Result<(), String> {
        let Some(directory) = &self.directory else { return Ok(()) };
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        fs::create_dir_all(directory).map_err(|e| format!("cannot create {}: {e}", directory.display()))?;
        let path = directory.join(name);
        let temporary = directory.join(format!("{name}.tmp"));
        // Written to the storage before it replaces the old file, so that a loss of power leaves
        // the old file or the new one, never an empty one.
        let mut file = fs::File::create(&temporary).map_err(|e| format!("cannot create {}: {e}", temporary.display()))?;
        file.write_all(&bytes).and_then(|_| file.sync_all()).map_err(|e| format!("cannot write {}: {e}", temporary.display()))?;
        drop(file);
        fs::rename(&temporary, &path).map_err(|e| format!("cannot replace {}: {e}", path.display()))?;
        // The rename itself is made durable by syncing the directory (not possible on every
        // system; a failure here leaves a complete file all the same).
        if let Ok(directory) = fs::File::open(directory) {
            let _ = directory.sync_all();
        }
        Ok(())
    }

    pub fn load_settings(&self) -> Result<Option<Settings>, String> {
        self.read(Self::SETTINGS)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        self.write(Self::SETTINGS, settings)
    }

    pub fn load_history(&self) -> Result<Option<History>, String> {
        self.read(Self::HISTORY)
    }

    pub fn save_history(&self, history: &History) -> Result<(), String> {
        self.write(Self::HISTORY, history)
    }
}
