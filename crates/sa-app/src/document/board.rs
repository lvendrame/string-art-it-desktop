//! Board configuration (specs 03, 04). Port of web `board.ts`.

use serde::{Deserialize, Serialize};

use sa_domain::shapes::{
    circle_shape, ellipse_shape, equilateral_triangle_shape, rectangle_shape,
    right_triangle_hypotenuse, right_triangle_shape, square_shape,
};
use sa_domain::{Path, Point};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardShape {
    Circle,
    Oval,
    Rectangle,
    Square,
    Triangle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriangleType {
    Equilateral,
    RightAngled,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardDimensions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f64,
    pub colour: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum BoardAppearance {
    Solid {
        colour: String,
    },
    LinearGradient {
        stops: Vec<GradientStop>,
        direction: f64,
    },
    RadialGradient {
        stops: Vec<GradientStop>,
        centre: Point,
    },
    #[serde(rename_all = "camelCase")]
    WoodTexture {
        preset_id: String,
    },
    #[serde(rename_all = "camelCase")]
    PaintedWood {
        preset_id: String,
    },
    /// Uploaded image stored inline as a data URL (spec 00 Phase 2).
    #[serde(rename_all = "camelCase")]
    CustomTexture {
        image_data_url: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub shape: BoardShape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triangle_type: Option<TriangleType>,
    pub dimensions: BoardDimensions,
    pub appearance: BoardAppearance,
}

const MIN_DIMENSION: f64 = 0.1;

pub fn clamp_dimension(value: f64) -> f64 {
    if value.is_finite() && value > MIN_DIMENSION {
        value
    } else {
        MIN_DIMENSION
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            shape: BoardShape::Circle,
            triangle_type: None,
            dimensions: BoardDimensions {
                diameter: Some(60.0),
                ..Default::default()
            },
            appearance: BoardAppearance::Solid {
                colour: "#3b3b3b".into(),
            },
        }
    }
}

/// Defaults applied when switching shape so stale fields never bleed across shapes.
pub fn default_dimensions_for(
    shape: BoardShape,
    triangle_type: Option<TriangleType>,
) -> BoardDimensions {
    let d = BoardDimensions::default;
    match shape {
        BoardShape::Circle => BoardDimensions {
            diameter: Some(60.0),
            ..d()
        },
        BoardShape::Oval | BoardShape::Rectangle => BoardDimensions {
            width: Some(60.0),
            height: Some(40.0),
            ..d()
        },
        BoardShape::Square => BoardDimensions {
            side: Some(50.0),
            ..d()
        },
        BoardShape::Triangle => match triangle_type {
            Some(TriangleType::RightAngled) => BoardDimensions {
                base: Some(40.0),
                height: Some(30.0),
                ..d()
            },
            _ => BoardDimensions {
                side: Some(50.0),
                ..d()
            },
        },
    }
}

/// The board is always centred on the document origin.
pub const BOARD_CENTER: Point = Point::ORIGIN;

impl Board {
    pub fn hypotenuse(&self) -> Option<f64> {
        (self.shape == BoardShape::Triangle
            && self.triangle_type == Some(TriangleType::RightAngled))
        .then(|| {
            right_triangle_hypotenuse(
                self.dimensions.base.unwrap_or(0.0),
                self.dimensions.height.unwrap_or(0.0),
            )
        })
    }

    pub fn path(&self) -> Path {
        let d = &self.dimensions;
        match self.shape {
            BoardShape::Circle => circle_shape(BOARD_CENTER, d.diameter.unwrap_or(60.0) / 2.0),
            BoardShape::Oval => ellipse_shape(
                BOARD_CENTER,
                d.width.unwrap_or(60.0) / 2.0,
                d.height.unwrap_or(40.0) / 2.0,
                0.0,
            ),
            BoardShape::Rectangle => {
                let (w, h) = (d.width.unwrap_or(60.0), d.height.unwrap_or(40.0));
                rectangle_shape(Point::new(-w / 2.0, -h / 2.0), w, h, 0.0)
            }
            BoardShape::Square => {
                let s = d.side.unwrap_or(50.0);
                square_shape(Point::new(-s / 2.0, -s / 2.0), s, 0.0)
            }
            BoardShape::Triangle => {
                if self.triangle_type == Some(TriangleType::RightAngled) {
                    let (b, h) = (d.base.unwrap_or(40.0), d.height.unwrap_or(30.0));
                    right_triangle_shape(Point::new(-b / 2.0, -h / 2.0), b, h)
                } else {
                    equilateral_triangle_shape(BOARD_CENTER, d.side.unwrap_or(50.0), 0.0)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_domain::transforms::bounding_box_of;

    #[test]
    fn default_board_is_60cm_circle() {
        let b = Board::default();
        assert_eq!(b.shape, BoardShape::Circle);
        assert!((b.path().length() - std::f64::consts::PI * 60.0).abs() < 1e-9);
    }

    #[test]
    fn clamp() {
        assert_eq!(clamp_dimension(-3.0), 0.1);
        assert_eq!(clamp_dimension(f64::NAN), 0.1);
        assert_eq!(clamp_dimension(12.0), 12.0);
    }

    #[test]
    fn right_triangle_hypotenuse_and_centring() {
        let b = Board {
            shape: BoardShape::Triangle,
            triangle_type: Some(TriangleType::RightAngled),
            dimensions: default_dimensions_for(
                BoardShape::Triangle,
                Some(TriangleType::RightAngled),
            ),
            appearance: BoardAppearance::Solid {
                colour: "#000".into(),
            },
        };
        assert_eq!(b.hypotenuse(), Some(50.0));
        let bb = bounding_box_of(&b.path().bounding_box_points());
        assert!((bb.min_x + 20.0).abs() < 1e-9 && (bb.max_y - 15.0).abs() < 1e-9);
        assert_eq!(Board::default().hypotenuse(), None);
    }

    #[test]
    fn json_shape_matches_web() {
        let b = Board {
            shape: BoardShape::Triangle,
            triangle_type: Some(TriangleType::RightAngled),
            dimensions: BoardDimensions {
                base: Some(40.0),
                height: Some(30.0),
                ..Default::default()
            },
            appearance: BoardAppearance::WoodTexture {
                preset_id: "oak".into(),
            },
        };
        let v = serde_json::to_value(&b).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "shape": "triangle",
                "triangleType": "right-angled",
                "dimensions": { "base": 40.0, "height": 30.0 },
                "appearance": { "type": "wood-texture", "presetId": "oak" }
            })
        );
        let custom: BoardAppearance =
            serde_json::from_str(r#"{"type":"custom-texture","imageDataUrl":"data:x"}"#).unwrap();
        assert_eq!(
            custom,
            BoardAppearance::CustomTexture {
                image_data_url: "data:x".into()
            }
        );
    }
}
