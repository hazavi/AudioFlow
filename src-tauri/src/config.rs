use crate::models::Config;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Storage {
    pub path: PathBuf,
}
impl Storage {
    pub fn new(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        // Preserve settings made in the initial development build when adopting
        // the final application identifier. Never overwrite an existing config.
        let destination = dir.join("config.json");
        if !destination.exists()
            && dir
                .file_name()
                .is_some_and(|n| n == "com.audioflow.desktop")
        {
            if let Some(parent) = dir.parent() {
                let legacy = parent.join("app.audioflow.desktop");
                if legacy.join("config.json").exists() {
                    let bytes = fs::read(legacy.join("config.json")).map_err(|e| e.to_string())?;
                    // Copy verbatim; load() validates and exposes recovery controls if invalid.
                    fs::write(&destination, &bytes).map_err(|e| e.to_string())?;
                    if legacy.join("activity.json").exists() {
                        let _ = fs::copy(legacy.join("activity.json"), dir.join("activity.json"));
                    }
                    if legacy.join("managed-routes.json").exists() {
                        let _ = fs::copy(
                            legacy.join("managed-routes.json"),
                            dir.join("managed-routes.json"),
                        );
                    }
                }
            }
        }
        Ok(Self {
            path: dir.join("config.json"),
        })
    }
    pub fn load(&self) -> Result<Config, String> {
        if !self.path.exists() {
            return Ok(Config::default());
        }
        let text = fs::read_to_string(&self.path).map_err(|e| e.to_string())?;
        let config: Config = serde_json::from_str(&text).map_err(|e| {
            format!("Configuration could not be read. The original file is preserved: {e}")
        })?;
        config.validate()?;
        Ok(config)
    }
    pub fn save(&self, config: &Config) -> Result<(), String> {
        config.validate()?;
        let temp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
        {
            use std::io::Write;
            let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        // MoveFileEx replaces atomically on Windows without deleting the old configuration first.
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            unsafe extern "system" {
                fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
            }
            let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = self.path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) } == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        #[cfg(not(windows))]
        fs::rename(temp, &self.path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_invalid_schema() {
        let dir = std::env::temp_dir().join(format!("audioflow-test-{}", uuid::Uuid::new_v4()));
        let storage = Storage::new(&dir).unwrap();
        let mut config = Config::default();
        config
            .assignments
            .insert("persistent-display".into(), "disconnected-endpoint".into());
        storage.save(&config).unwrap();
        assert_eq!(storage.load().unwrap().assignments, config.assignments);
        config.schema_version = 9;
        assert!(storage.save(&config).is_err());
        assert_eq!(storage.load().unwrap().schema_version, 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
