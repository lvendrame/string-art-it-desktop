//! Document model (spec 02).

pub mod board;
pub mod layers;
pub mod pin_path;
pub mod project;
pub mod symmetry;
pub mod thread_path;

pub use board::{Board, BoardAppearance, BoardDimensions, BoardShape, GradientStop, TriangleType};
pub use layers::{PinLayer, ThreadLayer};
pub use pin_path::{FontWeight, GeometryKind, Pin, PinPath, PinPathGeometry, PinStyle};
pub use project::{GridFile, ProjectError, ProjectFile};
pub use symmetry::{SYMMETRY_TYPE_CYCLE as SYMMETRY_TYPE_CYCLE_REF, SymmetryConfig, SymmetryType};
pub use thread_path::ThreadPath;

/// The undoable document content. Grid settings and editor state are not undoable
/// (spec 10) and live elsewhere.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub board: Board,
    pub pin_layers: Vec<PinLayer>,
    pub thread_layers: Vec<ThreadLayer>,
}

impl Document {
    /// Default board and one layer of each kind.
    pub fn empty() -> Self {
        Self {
            board: Board::default(),
            pin_layers: vec![PinLayer::new("Layer 1")],
            thread_layers: vec![ThreadLayer::new("Layer 1")],
        }
    }
}
