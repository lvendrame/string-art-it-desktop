//! Versioned project file (spec 16) — JSON-compatible with the web app's
//! `string-art-project.json`. Port of web `projectFile.ts`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Document;
use super::board::Board;
use super::layers::{PinLayer, ThreadLayer};
use crate::ids::seed_counter_from;

pub const CURRENT_PROJECT_VERSION: u32 = 1;

/// The persisted subset of grid settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridFile {
    pub gap_x: f64,
    pub gap_y: f64,
    pub colour: String,
    pub opacity: f64,
}

impl Default for GridFile {
    fn default() -> Self {
        Self {
            gap_x: 1.0,
            gap_y: 1.0,
            colour: "#6d5ef7".into(),
            opacity: 0.6,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub version: u32,
    pub board: Board,
    pub grid: GridFile,
    pub pin_layers: Vec<PinLayer>,
    pub thread_layers: Vec<ThreadLayer>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Not a valid StringArtIt project file.")]
    Invalid(#[source] Option<serde_json::Error>),
    #[error(
        "Project file version {0} is newer than this app supports (max {CURRENT_PROJECT_VERSION})."
    )]
    IncompatibleVersion(f64),
    #[error("No migration path from project version {0}.")]
    NoMigrationPath(f64),
}

/// Upgrades raw JSON from one version to the next.
type Migration = fn(Value) -> Value;

/// `(from_version, step)` pairs. Empty until a v2 schema exists.
const MIGRATIONS: &[(u32, Migration)] = &[];

impl ProjectFile {
    pub fn from_document(doc: &Document, grid: GridFile) -> Self {
        Self {
            version: CURRENT_PROJECT_VERSION,
            board: doc.board.clone(),
            grid,
            pin_layers: doc.pin_layers.clone(),
            thread_layers: doc.thread_layers.clone(),
        }
    }

    pub fn into_parts(self) -> (Document, GridFile) {
        (
            Document {
                board: self.board,
                pin_layers: self.pin_layers,
                thread_layers: self.thread_layers,
            },
            self.grid,
        )
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("project serializes")
    }

    /// Parses, migrates and type-checks a project file, then seeds the id
    /// counters past every id it contains.
    pub fn parse(text: &str) -> Result<Self, ProjectError> {
        let raw: Value = serde_json::from_str(text).map_err(|e| ProjectError::Invalid(Some(e)))?;
        let file = migrate(raw)?;
        file.seed_ids();
        Ok(file)
    }

    fn seed_ids(&self) {
        for l in &self.pin_layers {
            seed_counter_from(&l.id);
            for p in &l.pin_paths {
                seed_counter_from(&p.id);
                for pin in &p.pins {
                    seed_counter_from(&pin.id);
                }
            }
        }
        for l in &self.thread_layers {
            seed_counter_from(&l.id);
            for t in &l.thread_paths {
                seed_counter_from(&t.id);
            }
        }
    }
}

pub fn migrate(raw: Value) -> Result<ProjectFile, ProjectError> {
    let Some(obj) = raw.as_object() else {
        return Err(ProjectError::Invalid(None));
    };
    let Some(version) = obj.get("version") else {
        return Err(ProjectError::Invalid(None));
    };
    let mut version = match version {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.parse().unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    if version.is_nan() {
        return Err(ProjectError::Invalid(None));
    }
    if version > CURRENT_PROJECT_VERSION as f64 {
        return Err(ProjectError::IncompatibleVersion(version));
    }
    let mut data = raw;
    while version < CURRENT_PROJECT_VERSION as f64 {
        let step = MIGRATIONS
            .iter()
            .find(|(from, _)| *from as f64 == version)
            .ok_or(ProjectError::NoMigrationPath(version))?;
        data = (step.1)(data);
        version += 1.0;
    }
    if let Some(obj) = data.as_object_mut() {
        obj.insert("version".into(), Value::from(CURRENT_PROJECT_VERSION));
    }
    serde_json::from_value(data).map_err(|e| ProjectError::Invalid(Some(e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::pin_path::{PinPath, PinPathGeometry, PinStyle};
    use crate::document::symmetry::SymmetryConfig;
    use crate::document::thread_path::ThreadPath;
    use sa_domain::Point;

    #[test]
    fn round_trip() {
        let mut doc = Document::empty();
        doc.pin_layers[0].pin_paths.push(PinPath::new(
            PinPathGeometry::Circle {
                center: Point::ORIGIN,
                radius: 10.0,
            },
            1.0,
            &PinStyle {
                colour: "#fff".into(),
                diameter: 1.0,
                guide_visible: true,
            },
            SymmetryConfig::None,
        ));
        let pins = doc.pin_layers[0].pin_paths[0].pins.clone();
        doc.thread_layers[0].thread_paths.push(ThreadPath::new(
            vec![pins[0].id.clone(), pins[1].id.clone()],
            vec!["#f00".into()],
            1.0,
            6.0,
        ));
        doc.board.dimensions.diameter = Some(42.0);
        let grid = GridFile {
            gap_x: 2.0,
            gap_y: 3.0,
            colour: "#00ff00".into(),
            opacity: 0.25,
        };
        let text = ProjectFile::from_document(&doc, grid.clone()).to_json_pretty();
        let loaded = ProjectFile::parse(&text).unwrap();
        assert_eq!(loaded.version, CURRENT_PROJECT_VERSION);
        let (doc2, grid2) = loaded.into_parts();
        assert_eq!(doc2, doc);
        assert_eq!(grid2, grid);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(matches!(
            ProjectFile::parse("null"),
            Err(ProjectError::Invalid(_))
        ));
        assert!(matches!(
            ProjectFile::parse("\"x\""),
            Err(ProjectError::Invalid(_))
        ));
        assert!(matches!(
            ProjectFile::parse("42"),
            Err(ProjectError::Invalid(_))
        ));
        assert!(matches!(
            ProjectFile::parse(r#"{"board":{}}"#),
            Err(ProjectError::Invalid(_))
        ));
        assert!(matches!(
            ProjectFile::parse(r#"{"version":999}"#),
            Err(ProjectError::IncompatibleVersion(v)) if v == 999.0
        ));
        assert!(matches!(
            ProjectFile::parse(r#"{"version":0}"#),
            Err(ProjectError::NoMigrationPath(v)) if v == 0.0
        ));
        // Web gap fixed: wrong field types are rejected instead of loaded blindly.
        assert!(matches!(
            ProjectFile::parse(
                r#"{"version":1,"board":5,"grid":{},"pinLayers":[],"threadLayers":[]}"#
            ),
            Err(ProjectError::Invalid(_))
        ));
    }

    #[test]
    fn web_shaped_file_loads_and_seeds_ids() {
        let text = r##"{
  "version": 1,
  "board": { "shape": "circle", "dimensions": { "diameter": 60 }, "appearance": { "type": "solid", "colour": "#3b3b3b" } },
  "grid": { "gapX": 1, "gapY": 1, "colour": "#6d5ef7", "opacity": 0.6 },
  "pinLayers": [{ "id": "pinlayer-1", "name": "Layer 1", "visible": true, "locked": false, "pinPaths": [{
    "id": "pinpath-3", "geometry": { "type": "polygram", "center": { "x": 0, "y": 0 }, "radius": 10, "points": 5, "skip": 2, "rotation": 0 },
    "requestedSpacing": 1, "actualSpacing": 0.95, "pins": [{ "id": "pin-900", "x": 0, "y": -10 }],
    "guideVisible": true, "colour": "#d9d9d9", "diameter": 1.2, "symmetry": { "type": "both", "axis": { "x": 0, "y": 0 } }
  }] }],
  "threadLayers": [{ "id": "threadlayer-1", "name": "Layer 1", "visible": true, "locked": false, "threadPaths": [{
    "id": "threadpath-7", "colours": ["#ff0000", "#ffffff"], "width": 1, "pinIds": ["pin-900", "pin-900~mirror-1"], "twistPitch": 6
  }] }]
}"##;
        let f = ProjectFile::parse(text).unwrap();
        assert_eq!(
            f.pin_layers[0].pin_paths[0].symmetry,
            SymmetryConfig::Both {
                axis: Point::ORIGIN
            }
        );
        assert_eq!(crate::ids::next_id("pin"), "pin-901");
    }
}
