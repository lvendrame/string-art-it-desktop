//! Rendering: a backend-neutral [`scene::Scene`] built from the document, plus
//! backends — tiny-skia raster, SVG text and (D11) PDF.

pub mod color;
pub mod dash;
pub mod image;
pub mod path;
pub mod pdf;
pub mod raster;
pub mod scene;
pub mod svg;
pub mod text;
pub mod wood;

pub use color::Rgba;
pub use scene::{Scene, SceneOptions, Style, build_scene};
