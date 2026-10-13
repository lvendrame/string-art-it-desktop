//! Pure geometry for StringArtIt: paths, pin distribution, shapes, symmetry,
//! snapping, transforms, glyph flattening and generator index maths.
//! No IO, no UI — port of web `src/domain`.

pub mod generator;
pub mod lotus;
pub mod paths;
pub mod shapes;
pub mod snapping;
pub mod symmetry;
pub mod text;
pub mod transforms;

pub use paths::{Path, Point, Segment};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GeometryError {
    #[error("spacing must be positive")]
    NonPositiveSpacing,
    #[error("count must be positive")]
    NonPositiveCount,
    #[error("a polygon needs at least 3 sides")]
    TooFewSides,
    #[error("a star needs at least 3 points")]
    TooFewStarPoints,
    #[error("polygram {{{points}/{skip}}} is not a single continuous path (gcd != 1)")]
    PolygramNotContinuous { points: usize, skip: usize },
    #[error("radial interval must be positive")]
    NonPositiveRadialInterval,
}

#[cfg(test)]
pub(crate) mod test_util {
    #[track_caller]
    pub fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected} ± {tolerance}, got {actual}"
        );
    }
}
