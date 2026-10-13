//! The editor canvas (spec 05): toolbar, board viewport, status bar.
//! Pointer handling for each tool plugs in through [`CanvasInput`].

mod board_texture;
pub mod paint;

use egui::{Align2, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use sa_app::editor::{EditorMode, Store};
use sa_domain::Point;
use sa_domain::transforms::{
    BoundingBox, Viewport, bounding_box_of, fit_to_viewport, percent_to_zoom, zoom_at_point,
    zoom_to_percent,
};
use sa_render::scene::{Scene, SceneOptions, build_scene};

use crate::fonts;
use crate::icons::Icon;
use crate::ui::appearance::appearance_panel;
use crate::widgets::{self, Kit};
use board_texture::BoardTexture;
use paint::{Xf, c32};

pub const MIN_ZOOM_PERCENT: f64 = 5.0;
pub const MAX_ZOOM_PERCENT: f64 = 1600.0;
const ZOOM_STEP_FACTOR: f64 = 1.25;
const WHEEL_ZOOM_SENSITIVITY: f64 = 0.0022;
pub const ZOOM_PRESET_PERCENTS: [f64; 11] = [
    10.0, 25.0, 50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 400.0, 800.0, 1600.0,
];
const BOARD_MARGIN_CM: f64 = 5.0;
const FIT_PADDING_PX: f64 = 20.0;

fn clamp_zoom(z: f64) -> f64 {
    z.clamp(
        percent_to_zoom(MIN_ZOOM_PERCENT),
        percent_to_zoom(MAX_ZOOM_PERCENT),
    )
}

pub fn zoom_in_step(v: &Viewport, anchor: Point) -> Viewport {
    zoom_at_point(v, clamp_zoom(v.zoom * ZOOM_STEP_FACTOR), anchor)
}

pub fn zoom_out_step(v: &Viewport, anchor: Point) -> Viewport {
    zoom_at_point(v, clamp_zoom(v.zoom / ZOOM_STEP_FACTOR), anchor)
}

pub fn zoom_to_percent_step(v: &Viewport, percent: f64, anchor: Point) -> Viewport {
    zoom_at_point(
        v,
        percent_to_zoom(percent.clamp(MIN_ZOOM_PERCENT, MAX_ZOOM_PERCENT)),
        anchor,
    )
}

pub fn fit_viewport_for_board(store: &Store, size: egui::Vec2) -> Viewport {
    let b = bounding_box_of(&store.doc.board.path().bounding_box_points()).expand(BOARD_MARGIN_CM);
    fit_to_viewport(&b, size.x as f64, size.y as f64, FIT_PADDING_PX)
}

/// What the canvas reports to tool handlers each frame.
pub struct CanvasInput {
    pub response: egui::Response,
    pub xf: Xf,
    /// Document position under the pointer, if hovering the canvas.
    pub hover_doc: Option<Point>,
}

#[derive(Default)]
pub struct CanvasView {
    board_tex: BoardTexture,
    scene: Option<(u64, Scene)>,
    size: Option<egui::Vec2>,
    pan_start: Option<(Pos2, Point)>,
    pub cursor_doc: Option<Point>,
    origin: egui::Pos2,
    play_scene: Option<((u64, usize), Scene)>,
    /// Set to refit on the next frame (Continue, New, Open, Fit).
    pub fit_requested: bool,
}

impl CanvasView {
    pub fn request_fit(&mut self) {
        self.fit_requested = true;
    }

    pub fn zoom_in(&self, store: &mut Store) {
        store.editor.viewport = zoom_in_step(&store.editor.viewport, self.center());
    }

    pub fn zoom_out(&self, store: &mut Store) {
        store.editor.viewport = zoom_out_step(&store.editor.viewport, self.center());
    }

    /// Play mode (spec 19): read-only, statically framed, threads cut at
    /// `frame`. No toolbar, no pan/zoom.
    pub fn show_playback(&mut self, ui: &mut Ui, kit: &Kit, store: &Store, frame: usize) {
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        self.origin = rect.min;
        self.size = Some(rect.size());
        let vp = fit_viewport_for_board(store, rect.size());
        let xf = Xf {
            vp,
            origin: rect.min,
        };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, kit.t.bg_canvas);
        let ppp = ui.ctx().pixels_per_point() as f64;
        if let Some(tex) = self
            .board_tex
            .get(ui.ctx(), &store.doc.board, vp.zoom * ppp)
            .map(|t| t.id())
        {
            let b = self.board_tex.bbox;
            let r = Rect::from_min_max(
                xf.pos(Point::new(b.min_x, b.min_y)),
                xf.pos(Point::new(b.max_x, b.max_y)),
            );
            painter.image(
                tex,
                r,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        let key = (store.revision(), frame);
        if self.play_scene.as_ref().is_none_or(|(k, _)| *k != key) {
            let doc = sa_app::playback::document_at_frame(&store.doc, frame);
            self.play_scene = Some((key, build_scene(&doc, &SceneOptions::editor())));
        }
        let scene = &self.play_scene.as_ref().expect("built").1;
        let (board_items, content): (Vec<_>, Vec<_>) = scene
            .items
            .iter()
            .cloned()
            .partition(|i| matches!(i, sa_render::scene::Item::Board { .. }));
        paint::paint_items(
            &painter,
            &xf,
            &Scene {
                bounds: scene.bounds,
                items: board_items,
            },
        );
        if store.editor.grid.visible {
            draw_grid(&painter, &xf, rect, store);
        }
        paint::paint_items(
            &painter,
            &xf,
            &Scene {
                bounds: scene.bounds,
                items: content,
            },
        );
    }

    /// Top-left of the viewport area in screen points (last frame).
    pub fn origin(&self) -> egui::Pos2 {
        self.origin
    }

    /// Viewport area in screen points (last frame).
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.origin, self.size.unwrap_or_default())
    }

    fn center(&self) -> Point {
        let s = self.size.unwrap_or(vec2(720.0, 640.0));
        Point::new(s.x as f64 / 2.0, s.y as f64 / 2.0)
    }

    /// Shows toolbar, viewport and status bar. `tool` runs after the document is
    /// painted (to handle input and draw overlays) and may override the status.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        kit: &mut Kit,
        store: &mut Store,
        tool: impl FnOnce(&mut Ui, &mut Kit, &mut Store, &CanvasInput) -> Option<String>,
    ) {
        let t = kit.t;
        // Toolbar
        egui::Frame::new()
            .fill(t.bg_panel)
            .inner_margin(egui::Margin::symmetric(16, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| self.toolbar(ui, kit, store));
            });
        let line = |ui: &mut Ui| {
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().rect_filled(r, 0.0, t.border);
        };
        line(ui);

        let status_h = 32.0;
        let avail = ui.available_size();
        let area_size = vec2(avail.x, (avail.y - status_h - 1.0).max(50.0));
        let (rect, response) = ui.allocate_exact_size(area_size, Sense::click_and_drag());
        let input = self.viewport_area(ui, kit, store, rect, response);
        let status_override = if store.editor.mode == EditorMode::Pan {
            None
        } else {
            tool(ui, kit, store, &input)
        };
        line(ui);
        let status = status_override.unwrap_or_else(|| self.status_text(kit, store));
        let (sr, _) = ui.allocate_exact_size(vec2(avail.x, status_h), Sense::hover());
        ui.painter().rect_filled(sr, 0.0, t.bg_panel);
        ui.painter().text(
            pos2(sr.min.x + 16.0, sr.center().y),
            Align2::LEFT_CENTER,
            status,
            fonts::mono(11.5),
            t.text_secondary,
        );
    }

    fn toolbar(&mut self, ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
        ui.spacing_mut().item_spacing.x = 10.0;
        let g = store.editor.grid.clone();
        let label = kit.tr(if g.visible {
            "toolbars:canvasToolbar.gridOn"
        } else {
            "toolbars:canvasToolbar.gridOff"
        });
        if widgets::chip(ui, kit, Icon::Grid3x3, &label, g.visible).clicked() {
            store.editor.grid.visible = !g.visible;
        }
        let label = kit.tr(if g.snap_enabled {
            "toolbars:canvasToolbar.snapOn"
        } else {
            "toolbars:canvasToolbar.snapOff"
        });
        if widgets::chip(ui, kit, Icon::Magnet, &label, g.snap_enabled).clicked() {
            store.editor.grid.snap_enabled = !g.snap_enabled;
        }

        // Grid settings popover
        let tip = kit.tr("toolbars:canvasToolbar.gridSettings");
        let open_id = ui.id().with("grid-popover");
        let open = egui::Popup::is_id_open(ui.ctx(), open_id);
        let resp =
            widgets::button(ui, kit, Some(Icon::Settings), None, open, true).on_hover_text(&tip);
        egui::Popup::from_toggle_button_response(&resp)
            .id(open_id)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(220.0);
                let grid = &mut store.editor.grid;
                widgets::property_row(ui, kit, &kit.tr("toolbars:canvasToolbar.gapX"), |ui| {
                    if let Some(v) = widgets::number(ui, kit, grid.gap_x, 0.1..=1000.0, 0.05, 2) {
                        grid.gap_x = v.max(0.1);
                    }
                });
                widgets::property_row(ui, kit, &kit.tr("toolbars:canvasToolbar.gapY"), |ui| {
                    if let Some(v) = widgets::number(ui, kit, grid.gap_y, 0.1..=1000.0, 0.05, 2) {
                        grid.gap_y = v.max(0.1);
                    }
                });
                widgets::property_row(
                    ui,
                    kit,
                    &kit.tr("toolbars:canvasToolbar.gridColour"),
                    |ui| {
                        if let Some(c) = widgets::colour(ui, &grid.colour) {
                            grid.colour = c;
                        }
                    },
                );
                widgets::property_row(
                    ui,
                    kit,
                    &kit.tr("toolbars:canvasToolbar.gridOpacity"),
                    |ui| {
                        if let Some(v) = widgets::number(ui, kit, grid.opacity, 0.0..=1.0, 0.01, 2)
                        {
                            grid.opacity = v.clamp(0.0, 1.0);
                        }
                    },
                );
            });

        // Change background popover
        let tip = kit.tr("toolbars:canvasToolbar.changeBackground");
        let bg_id = ui.id().with("bg-popover");
        let open = egui::Popup::is_id_open(ui.ctx(), bg_id);
        let resp =
            widgets::button(ui, kit, Some(Icon::Wallpaper), None, open, true).on_hover_text(&tip);
        egui::Popup::from_toggle_button_response(&resp)
            .id(bg_id)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(260.0);
                appearance_panel(ui, kit, store);
            });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let fit_label = kit.tr("toolbars:canvasToolbar.fit");
            if widgets::button(ui, kit, Some(Icon::Maximize), Some(&fit_label), false, true)
                .clicked()
            {
                self.request_fit();
            }
            let c = self.center();
            if widgets::button(ui, kit, Some(Icon::ZoomIn), None, false, true)
                .on_hover_text(kit.tr("toolbars:canvasToolbar.zoomIn"))
                .clicked()
            {
                store.editor.viewport = zoom_in_step(&store.editor.viewport, c);
            }
            self.zoom_control(ui, kit, store);
            if widgets::button(ui, kit, Some(Icon::ZoomOut), None, false, true)
                .on_hover_text(kit.tr("toolbars:canvasToolbar.zoomOut"))
                .clicked()
            {
                store.editor.viewport = zoom_out_step(&store.editor.viewport, c);
            }
        });
    }

    /// Typed zoom % plus preset dropdown (web ZoomControl).
    fn zoom_control(&mut self, ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
        let current = zoom_to_percent(store.editor.viewport.zoom).round();
        let c = self.center();
        let presets_id = ui.id().with("zoom-presets");
        let open = egui::Popup::is_id_open(ui.ctx(), presets_id);
        let chevron = widgets::button(ui, kit, Some(Icon::ChevronDown), None, open, true)
            .on_hover_text(kit.tr("toolbars:canvasToolbar.zoomOptions"));
        egui::Popup::from_toggle_button_response(&chevron)
            .id(presets_id)
            .show(|ui| {
                for p in ZOOM_PRESET_PERCENTS {
                    let label = egui::RichText::new(format!("{p}%")).font(fonts::mono(12.0));
                    if ui.selectable_label(current == p, label).clicked() {
                        store.editor.viewport = zoom_to_percent_step(&store.editor.viewport, p, c);
                        ui.close();
                    }
                }
            });
        let mut v = current;
        let resp = ui
            .add_sized(
                vec2(64.0, 30.0),
                egui::DragValue::new(&mut v)
                    .range(MIN_ZOOM_PERCENT..=MAX_ZOOM_PERCENT)
                    .speed(1.0)
                    .suffix("%")
                    .max_decimals(0),
            )
            .on_hover_text(kit.tr("toolbars:canvasToolbar.zoomLevel"));
        if resp.changed() && v != current {
            store.editor.viewport = zoom_to_percent_step(&store.editor.viewport, v, c);
        }
    }

    fn viewport_area(
        &mut self,
        ui: &mut Ui,
        kit: &mut Kit,
        store: &mut Store,
        rect: Rect,
        response: egui::Response,
    ) -> CanvasInput {
        let t = kit.t;
        // Size tracking: first measurement fits; later resizes keep the centre fixed.
        let size = rect.size();
        match self.size {
            None => self.fit_requested = true,
            Some(prev) if prev != size => {
                let v = &mut store.editor.viewport;
                v.pan_origin.x -= (size.x - prev.x) as f64 / 2.0 / v.zoom;
                v.pan_origin.y -= (size.y - prev.y) as f64 / 2.0 / v.zoom;
            }
            _ => {}
        }
        self.size = Some(size);
        if self.fit_requested {
            store.editor.viewport = fit_viewport_for_board(store, size);
            self.fit_requested = false;
        }

        // Pan: middle-drag anywhere, primary-drag in Pan mode.
        let pointer = ui.input(|i| i.pointer.clone());
        let pan_button = response.dragged_by(egui::PointerButton::Middle)
            || (store.editor.mode == EditorMode::Pan
                && response.dragged_by(egui::PointerButton::Primary));
        if pan_button {
            if let Some(pos) = pointer.interact_pos() {
                let (start, origin) = *self
                    .pan_start
                    .get_or_insert((pos, store.editor.viewport.pan_origin));
                let v = &mut store.editor.viewport;
                v.pan_origin = Point::new(
                    origin.x - (pos.x - start.x) as f64 / v.zoom,
                    origin.y - (pos.y - start.y) as f64 / v.zoom,
                );
            }
        } else {
            self.pan_start = None;
        }

        // Wheel / pinch zoom anchored at the cursor.
        if let Some(hover) = response.hover_pos() {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let anchor = Point::new((hover.x - rect.min.x) as f64, (hover.y - rect.min.y) as f64);
            let v = &store.editor.viewport;
            let mut z = v.zoom;
            if scroll != 0.0 {
                z *= (scroll as f64 * WHEEL_ZOOM_SENSITIVITY).exp();
            }
            if pinch != 1.0 {
                z *= pinch as f64;
            }
            if z != v.zoom {
                store.editor.viewport = zoom_at_point(v, clamp_zoom(z), anchor);
            }
        }

        let xf = Xf {
            vp: store.editor.viewport,
            origin: rect.min,
        };
        self.origin = rect.min;
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, t.bg_canvas);

        // Board fill texture
        let ppp = ui.ctx().pixels_per_point() as f64;
        if let Some(tex) = self
            .board_tex
            .get(ui.ctx(), &store.doc.board, xf.vp.zoom * ppp)
            .map(|t| t.id())
        {
            let b = self.board_tex.bbox;
            let r = Rect::from_min_max(
                xf.pos(Point::new(b.min_x, b.min_y)),
                xf.pos(Point::new(b.max_x, b.max_y)),
            );
            painter.image(
                tex,
                r,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        // Board outline stroke drawn first, then grid dots, then the content.
        let revision = store.revision();
        if self.scene.as_ref().is_none_or(|(r, _)| *r != revision) {
            self.scene = Some((revision, build_scene(&store.doc, &SceneOptions::editor())));
        }
        let scene = &self.scene.as_ref().expect("built").1;
        let (board_items, content): (Vec<_>, Vec<_>) = scene
            .items
            .iter()
            .cloned()
            .partition(|i| matches!(i, sa_render::scene::Item::Board { .. }));
        paint::paint_items(
            &painter,
            &xf,
            &Scene {
                bounds: scene.bounds,
                items: board_items,
            },
        );
        if store.editor.grid.visible {
            draw_grid(&painter, &xf, rect, store);
        }
        paint::paint_items(
            &painter,
            &xf,
            &Scene {
                bounds: scene.bounds,
                items: content,
            },
        );

        let hover_doc = response.hover_pos().map(|p| xf.doc(p));
        self.cursor_doc = hover_doc;
        if store.editor.mode == EditorMode::Pan {
            ui.ctx().set_cursor_icon(if self.pan_start.is_some() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        CanvasInput {
            response,
            xf,
            hover_doc,
        }
    }

    fn status_text(&self, kit: &Kit, store: &Store) -> String {
        let zoom = zoom_to_percent(store.editor.viewport.zoom).round();
        match store.editor.mode {
            EditorMode::Pin => {
                let tool = store.editor.pin_tool;
                let action = if matches!(
                    tool,
                    sa_app::editor::PinTool::Ellipse
                        | sa_app::editor::PinTool::Circle
                        | sa_app::editor::PinTool::Rectangle
                        | sa_app::editor::PinTool::Square
                ) {
                    kit.tr("canvas:status.clickDragAction")
                } else {
                    kit.tr("canvas:status.clickAction")
                };
                kit.trf(
                    "canvas:status.pinToolHint",
                    &[("tool", &tool.key()), ("action", &action)],
                )
            }
            EditorMode::Select | EditorMode::Pan | EditorMode::Generate => match self.cursor_doc {
                Some(p) => kit.trf(
                    "canvas:status.cursor",
                    &[
                        ("x", &format!("{:.1}", p.x)),
                        ("y", &format!("{:.1}", p.y)),
                        ("zoom", &zoom),
                    ],
                ),
                None => kit.trf("canvas:status.zoom", &[("zoom", &zoom)]),
            },
            _ => kit.tr("canvas:status.threadDefault"),
        }
    }
}

/// Dot grid (web GridLayer): one dot per cell at (gap·i + r, gap·j + r).
fn draw_grid(painter: &egui::Painter, xf: &Xf, rect: Rect, store: &Store) {
    let g = &store.editor.grid;
    let r = (g.gap_x.min(g.gap_y) * 0.06).max(0.03);
    let r_px = xf.len(r);
    let gap_px = (g.gap_x.min(g.gap_y) * xf.vp.zoom) as f32;
    if gap_px < 3.0 {
        return; // too dense to be meaningful; avoids drawing millions of dots
    }
    let color = c32(sa_render::Rgba::parse(&g.colour).with_opacity(g.opacity));
    let tl = xf.doc(rect.min);
    let br = xf.doc(rect.max);
    let i0 = (tl.x / g.gap_x).floor() as i64 - 1;
    let i1 = (br.x / g.gap_x).ceil() as i64;
    let j0 = (tl.y / g.gap_y).floor() as i64 - 1;
    let j1 = (br.y / g.gap_y).ceil() as i64;
    for i in i0..=i1 {
        for j in j0..=j1 {
            let p = xf.pos(Point::new(i as f64 * g.gap_x + r, j as f64 * g.gap_y + r));
            painter.circle_filled(p, r_px.max(0.6), color);
        }
    }
}

/// Snap indicator ring (spec 05 §Grid Snap Feedback).
pub fn draw_snap_indicator(painter: &egui::Painter, xf: &Xf, p: Point, accent: egui::Color32) {
    let c = xf.pos(p);
    painter.circle_stroke(c, xf.len(0.3), Stroke::new(xf.len(0.05).max(1.0), accent));
    painter.circle_filled(c, xf.len(0.06).max(1.5), accent);
}

#[allow(dead_code)]
pub fn bbox_rect(xf: &Xf, b: &BoundingBox) -> Rect {
    Rect::from_min_max(
        xf.pos(Point::new(b.min_x, b.min_y)),
        xf.pos(Point::new(b.max_x, b.max_y)),
    )
}
