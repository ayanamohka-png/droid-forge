use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const EMBEDDED_DEVICES_JSON: &str = include_str!("../devices.json");

#[derive(Debug, Clone, Deserialize)]
pub struct KnownDevice {
    pub brand: String,
    pub model: String,
    pub codename: String,
    pub soc: String,
}

pub struct DeviceDatabase {
    entries: Vec<KnownDevice>,
}

impl DeviceDatabase {
    pub fn load() -> Self {
        let external = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join("devices.json")))
            .or_else(|| Some(PathBuf::from("devices.json")))
            .and_then(|path| fs::read_to_string(path).ok());

        let entries = external
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Vec<KnownDevice>>(raw).ok())
            .or_else(|| serde_json::from_str::<Vec<KnownDevice>>(EMBEDDED_DEVICES_JSON).ok())
            .unwrap_or_default();

        Self { entries }
    }

    pub fn lookup(&self, codename: &str) -> Option<&KnownDevice> {
        let needle = codename.trim();
        if needle.is_empty() {
            return None;
        }
        self.entries
            .iter()
            .find(|d| d.codename.eq_ignore_ascii_case(needle))
    }
}
