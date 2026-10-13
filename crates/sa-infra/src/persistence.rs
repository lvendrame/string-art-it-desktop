//! Desktop persistence (spec 16): project files, autosave and preferences in
//! the OS app-data/config directories (web used localStorage).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use sa_app::document::{ProjectError, ProjectFile};

fn dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("com", "StringArtIt", "StringArtIt")
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Project(#[from] ProjectError),
}

pub fn read_project(path: &Path) -> Result<ProjectFile, FileError> {
    Ok(ProjectFile::parse(&std::fs::read_to_string(path)?)?)
}

/// Atomic-ish write: temp file + rename, so a crash never truncates a project.
pub fn write_project(path: &Path, file: &ProjectFile) -> Result<(), FileError> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, file.to_json_pretty())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub struct Autosave {
    path: Option<PathBuf>,
}

impl Default for Autosave {
    fn default() -> Self {
        Self {
            path: dirs().map(|d| d.data_dir().join("autosave.v1.json")),
        }
    }
}

impl Autosave {
    pub fn at(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// Best effort, like the web localStorage autosave.
    pub fn save(&self, file: &ProjectFile) {
        let Some(p) = &self.path else { return };
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = write_project(p, file) {
            log::warn!("autosave failed: {e}");
        }
    }

    pub fn load(&self) -> Option<ProjectFile> {
        read_project(self.path.as_ref()?).ok()
    }

    pub fn clear(&self) {
        if let Some(p) = &self.path {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    pub language: Option<String>,
    /// Print calibration correction factor (spec 14).
    pub calibration_factor: Option<f64>,
    pub last_dir: Option<PathBuf>,
    /// Local MCP agent server (spec 31 desktop): off unless opted in.
    #[serde(default)]
    pub agent_server: bool,
    #[serde(default)]
    pub agent_port: Option<u16>,
}

impl Prefs {
    fn path() -> Option<PathBuf> {
        dirs().map(|d| d.config_dir().join("prefs.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(p) = Self::path() else { return };
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_app::editor::Store;

    #[test]
    fn autosave_round_trip() {
        let dir = std::env::temp_dir().join(format!("sa-autosave-{}", std::process::id()));
        let a = Autosave::at(dir.join("autosave.json"));
        let file = Store::default().to_project_file();
        a.save(&file);
        assert_eq!(a.load().unwrap(), file);
        a.clear();
        assert!(a.load().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
