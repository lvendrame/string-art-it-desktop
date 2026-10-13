//! Canvas tool controllers: translate pointer/keyboard input into store
//! operations and draw tool overlays (previews, highlights, hover states).

pub mod pin;
pub mod select;
pub mod thread;

use egui::{Painter, Pos2, Stroke};
use sa_app::editor::Store;
use sa_domain::Point;
use sa_domain::snapping::{GridGap, SnapOptions, SnapPin, SnapResult, resolve_snap_position};

use crate::canvas::paint::Xf;

/// Pointer state for one frame, in document space.
pub struct Pointer {
    pub raw: Option<Point>,
    pub pressed: bool,
    pub released: bool,
    pub down: bool,
    pub double_clicked: bool,
    pub alt: bool,
    pub command: bool,
    pub shift: bool,
}

impl Pointer {
    pub fn read(ui: &egui::Ui, input: &crate::canvas::CanvasInput) -> Self {
        let hovered = input.response.hovered() || input.response.is_pointer_button_down_on();
        ui.input(|i| Pointer {
            raw: input.hover_doc,
            pressed: hovered && i.pointer.primary_pressed(),
            released: i.pointer.primary_released(),
            down: i.pointer.primary_down(),
            double_clicked: hovered
                && i.pointer
                    .button_double_clicked(egui::PointerButton::Primary),
            alt: i.modifiers.alt,
            command: i.modifiers.command,
            shift: i.modifiers.shift,
        })
    }
}

/// Snap pins: every real pin on every layer (web useSnappedPointer).
pub fn snap_pins(store: &Store) -> Vec<SnapPin> {
    store
        .doc
        .pin_layers
        .iter()
        .flat_map(|l| &l.pin_paths)
        .flat_map(|p| &p.pins)
        .map(|p| SnapPin {
            id: p.id.clone(),
            point: p.point(),
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapSource {
    Center,
    Pin,
    Grid,
    Raw,
}

pub fn snap(
    store: &Store,
    pins: &[SnapPin],
    raw: Point,
    center: Option<Point>,
) -> (Point, SnapSource) {
    let e = &store.editor;
    let opts = SnapOptions {
        pins,
        pin_snap_enabled: e.snap.pin_snap_enabled,
        snap_radius_px: e.snap.radius_px,
        grid_snap_enabled: e.grid.snap_enabled,
        grid_gap: GridGap {
            x: e.grid.gap_x,
            y: e.grid.gap_y,
        },
        viewport: e.viewport,
        center_snap: center,
    };
    let r = resolve_snap_position(raw, &opts);
    let src = match r {
        SnapResult::Center(_) => SnapSource::Center,
        SnapResult::Pin(_) => SnapSource::Pin,
        SnapResult::Grid(_) => SnapSource::Grid,
        SnapResult::Raw(_) => SnapSource::Raw,
    };
    (r.point(), src)
}

/// Max hit distance in document units for the configured snap radius.
pub fn hit_radius(store: &Store) -> f64 {
    store.editor.snap.radius_px / store.editor.viewport.zoom
}

pub fn dashed(
    painter: &Painter,
    xf: &Xf,
    points: &[Point],
    width_cm: f64,
    dash: [f64; 2],
    color: egui::Color32,
) {
    let stroke = sa_render::scene::Stroke {
        color: sa_render::Rgba {
            r: color.r(),
            g: color.g(),
            b: color.b(),
            a: color.a(),
        },
        width: width_cm,
        dash: Some(sa_render::scene::Dash {
            array: dash.to_vec(),
            offset: 0.0,
        }),
    };
    crate::canvas::paint::stroke_polyline(painter, xf, points, &stroke);
}

pub fn ring(painter: &Painter, xf: &Xf, p: Point, r_cm: f64, w_cm: f64, color: egui::Color32) {
    painter.circle_stroke(
        xf.pos(p),
        xf.len(r_cm),
        Stroke::new(xf.len(w_cm).max(1.0), color),
    );
}

pub fn pos(xf: &Xf, p: Point) -> Pos2 {
    xf.pos(p)
}
