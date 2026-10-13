//! Snap pipeline (spec 05 §Snapping). Port of web `src/domain/snapping/*`.

use crate::paths::Point;
use crate::transforms::{Viewport, screen_distance_to_document};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGap {
    pub x: f64,
    pub y: f64,
}

pub fn snap_to_grid(point: Point, gap: GridGap) -> Point {
    // JS Math.round rounds .5 toward +∞; f64::round rounds away from zero.
    let js_round = |v: f64| (v + 0.5).floor();
    Point::new(
        js_round(point.x / gap.x) * gap.x,
        js_round(point.y / gap.y) * gap.y,
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapPin {
    pub id: String,
    pub point: Point,
}

/// Nearest pin within `max_document_distance` (already converted from the
/// screen-space radius by the caller).
pub fn find_nearest_pin(
    point: Point,
    pins: &[SnapPin],
    max_document_distance: f64,
) -> Option<&SnapPin> {
    let mut nearest = None;
    let mut nearest_distance = f64::INFINITY;
    for pin in pins {
        let d = pin.point.distance(point);
        if d <= max_document_distance && d < nearest_distance {
            nearest = Some(pin);
            nearest_distance = d;
        }
    }
    nearest
}

pub struct SnapOptions<'a> {
    pub pins: &'a [SnapPin],
    pub pin_snap_enabled: bool,
    pub snap_radius_px: f64,
    pub grid_snap_enabled: bool,
    pub grid_gap: GridGap,
    pub viewport: Viewport,
    pub center_snap: Option<Point>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SnapResult<'a> {
    Center(Point),
    Pin(&'a SnapPin),
    Grid(Point),
    Raw(Point),
}

impl SnapResult<'_> {
    pub fn point(&self) -> Point {
        match self {
            SnapResult::Center(p) | SnapResult::Grid(p) | SnapResult::Raw(p) => *p,
            SnapResult::Pin(pin) => pin.point,
        }
    }
}

/// Fixed priority: centre override (Ctrl/Cmd) → nearest pin → grid → raw.
/// Grid visibility and snapping are independent; only `grid_snap_enabled` is read.
pub fn resolve_snap_position<'a>(raw: Point, options: &SnapOptions<'a>) -> SnapResult<'a> {
    if let Some(center) = options.center_snap {
        return SnapResult::Center(center);
    }
    if options.pin_snap_enabled {
        let max = screen_distance_to_document(options.snap_radius_px, &options.viewport);
        if let Some(pin) = find_nearest_pin(raw, options.pins, max) {
            return SnapResult::Pin(pin);
        }
    }
    if options.grid_snap_enabled {
        return SnapResult::Grid(snap_to_grid(raw, options.grid_gap));
    }
    SnapResult::Raw(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(id: &str, x: f64, y: f64) -> SnapPin {
        SnapPin {
            id: id.into(),
            point: Point::new(x, y),
        }
    }

    #[test]
    fn grid() {
        assert_eq!(
            snap_to_grid(Point::new(1.2, 1.8), GridGap { x: 1.0, y: 1.0 }),
            Point::new(1.0, 2.0)
        );
    }

    #[test]
    fn nearest_pin() {
        let pins = [pin("a", 0.0, 0.0), pin("b", 10.0, 0.0)];
        assert_eq!(
            find_nearest_pin(Point::new(1.0, 0.0), &pins, 5.0)
                .unwrap()
                .id,
            "a"
        );
        assert!(find_nearest_pin(Point::new(4.9, 0.0), &pins, 2.0).is_none());
    }

    fn opts<'a>(
        pins: &'a [SnapPin],
        pin_snap: bool,
        grid_snap: bool,
        zoom: f64,
        radius: f64,
    ) -> SnapOptions<'a> {
        SnapOptions {
            pins,
            pin_snap_enabled: pin_snap,
            snap_radius_px: radius,
            grid_snap_enabled: grid_snap,
            grid_gap: GridGap { x: 1.0, y: 1.0 },
            viewport: Viewport {
                zoom,
                pan_origin: Point::ORIGIN,
            },
            center_snap: None,
        }
    }

    #[test]
    fn priority() {
        let pins = [pin("p1", 5.02, 5.02)];
        let mut o = opts(&pins, true, true, 2.0, 20.0);
        o.center_snap = Some(Point::ORIGIN);
        assert_eq!(
            resolve_snap_position(Point::new(5.0, 5.0), &o),
            SnapResult::Center(Point::ORIGIN)
        );

        let o = opts(&pins, true, true, 2.0, 20.0);
        assert_eq!(
            resolve_snap_position(Point::new(5.0, 5.0), &o),
            SnapResult::Pin(&pins[0])
        );

        let o = opts(&[], true, true, 2.0, 20.0);
        assert_eq!(
            resolve_snap_position(Point::new(5.4, 5.4), &o),
            SnapResult::Grid(Point::new(5.0, 5.0))
        );

        let o = opts(&[], false, false, 2.0, 20.0);
        assert_eq!(
            resolve_snap_position(Point::new(5.4, 5.4), &o),
            SnapResult::Raw(Point::new(5.4, 5.4))
        );
    }

    #[test]
    fn radius_is_screen_space() {
        let pins = [pin("p1", 5.4, 5.0)];
        let low = opts(&pins, true, false, 1.0, 10.0);
        let high = opts(&pins, true, false, 100.0, 10.0);
        assert!(matches!(
            resolve_snap_position(Point::new(5.0, 5.0), &low),
            SnapResult::Pin(_)
        ));
        assert!(matches!(
            resolve_snap_position(Point::new(5.0, 5.0), &high),
            SnapResult::Raw(_)
        ));
    }
}
