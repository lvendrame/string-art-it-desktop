//! tiny-skia backend: renders a [`Scene`] to RGBA pixels. Used for PNG/JPEG
//! export, video frames, the editor's board-fill texture and print previews.

use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Mask, Paint, PathBuilder, Pattern, Pixmap,
    RadialGradient, Shader, SpreadMode, Stroke as SkStroke, StrokeDash, Transform,
};

use sa_domain::Point;
use sa_domain::transforms::BoundingBox;

use crate::color::Rgba;
use crate::path::{PathCmd, PathData};
use crate::scene::{BoardFill, Item, Scene, Stop, Stroke};
use crate::text::centered_text_outline;

/// Maps scene centimetres to pixels: `px = (cm - origin) * scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub origin: Point,
    pub scale: f64,
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    /// Whole scene at `dpi` (physical size × DPI, spec 15).
    pub fn for_scene_at_dpi(scene: &Scene, dpi: f64) -> Self {
        Self::for_bounds(&scene.bounds, dpi / 2.54)
    }

    pub fn for_bounds(bounds: &BoundingBox, px_per_cm: f64) -> Self {
        Self {
            origin: Point::new(bounds.min_x, bounds.min_y),
            scale: px_per_cm,
            width: (bounds.width() * px_per_cm).round().max(1.0) as u32,
            height: (bounds.height() * px_per_cm).round().max(1.0) as u32,
        }
    }

    fn transform(&self) -> Transform {
        let s = self.scale as f32;
        Transform::from_row(
            s,
            0.0,
            0.0,
            s,
            -self.origin.x as f32 * s,
            -self.origin.y as f32 * s,
        )
    }
}

fn color(c: Rgba) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, c.a)
}

fn solid(c: Rgba) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(color(c));
    p.anti_alias = true;
    p
}

pub fn to_sk_path(d: &PathData) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    for cmd in &d.cmds {
        match *cmd {
            PathCmd::MoveTo(p) => pb.move_to(p.x as f32, p.y as f32),
            PathCmd::LineTo(p) => pb.line_to(p.x as f32, p.y as f32),
            PathCmd::CubicTo(a, b, p) => pb.cubic_to(
                a.x as f32, a.y as f32, b.x as f32, b.y as f32, p.x as f32, p.y as f32,
            ),
            PathCmd::Close => pb.close(),
        }
    }
    pb.finish()
}

fn sk_stroke(s: &Stroke) -> SkStroke {
    SkStroke {
        width: s.width as f32,
        dash: s.dash.as_ref().and_then(|d| {
            let mut arr: Vec<f32> = d.array.iter().map(|v| *v as f32).collect();
            if arr.len() % 2 == 1 {
                arr.extend_from_within(..);
            }
            StrokeDash::new(arr, d.offset as f32)
        }),
        ..SkStroke::default()
    }
}

fn stops(s: &[Stop]) -> Vec<GradientStop> {
    s.iter()
        .map(|s| GradientStop::new(s.offset as f32, color(s.color)))
        .collect()
}

/// Unit square → bbox.
fn bbox_transform(b: &BoundingBox) -> Transform {
    Transform::from_row(
        b.width() as f32,
        0.0,
        0.0,
        b.height() as f32,
        b.min_x as f32,
        b.min_y as f32,
    )
}

fn draw_board(
    pixmap: &mut Pixmap,
    vp: &Viewport,
    outline: &PathData,
    bbox: &BoundingBox,
    fill: &BoardFill,
) {
    let Some(path) = to_sk_path(outline) else {
        return;
    };
    let ts = vp.transform();
    let unit = ts.pre_concat(bbox_transform(bbox));
    match fill {
        BoardFill::Solid(c) => pixmap.fill_path(&path, &solid(*c), FillRule::Winding, ts, None),
        BoardFill::Linear {
            stops: s,
            direction_deg,
        } => {
            let shader = LinearGradient::new(
                tiny_skia::Point::from_xy(0.0, 0.0),
                tiny_skia::Point::from_xy(1.0, 0.0),
                stops(s),
                SpreadMode::Pad,
                bbox_transform(bbox).pre_concat(Transform::from_rotate(*direction_deg as f32)),
            );
            fill_with_shader(pixmap, &path, shader, ts);
        }
        BoardFill::Radial { stops: s, centre } => {
            let c = tiny_skia::Point::from_xy(centre.x as f32, centre.y as f32);
            let shader = RadialGradient::new(
                c,
                0.0,
                c,
                0.5,
                stops(s),
                SpreadMode::Pad,
                bbox_transform(bbox),
            );
            fill_with_shader(pixmap, &path, shader, ts);
        }
        BoardFill::Wood { bands, tint } => {
            let Some(mask) = clip_mask(vp, &path) else {
                return;
            };
            // Painter's algorithm, outermost first: each band's outer contour as a
            // solid disk. Same coverage as the even-odd annuli but without
            // anti-aliasing seams between neighbouring bands.
            for band in bands.iter().rev() {
                if let Some(p) = to_sk_path(&PathData::polyline(&band.outer, true)) {
                    pixmap.fill_path(
                        &p,
                        &solid(band.colour),
                        FillRule::Winding,
                        unit,
                        Some(&mask),
                    );
                }
            }
            if let Some(t) = tint {
                let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).expect("unit rect");
                pixmap.fill_rect(rect, &solid(*t), unit, Some(&mask));
            }
        }
        BoardFill::Image(img) => {
            let Some(src) = image_pixmap(img) else { return };
            // xMidYMid slice: cover the bbox, centred.
            let s = (bbox.width() / img.width as f64).max(bbox.height() / img.height as f64);
            let tx = bbox.min_x + (bbox.width() - img.width as f64 * s) / 2.0;
            let ty = bbox.min_y + (bbox.height() - img.height as f64 * s) / 2.0;
            let shader = Pattern::new(
                src.as_ref(),
                SpreadMode::Pad,
                tiny_skia::FilterQuality::Bicubic,
                1.0,
                Transform::from_row(s as f32, 0.0, 0.0, s as f32, tx as f32, ty as f32),
            );
            let mut paint = Paint {
                shader,
                anti_alias: true,
                ..Paint::default()
            };
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::Winding, ts, None);
        }
    }
}

fn image_pixmap(img: &crate::image::DecodedImage) -> Option<Pixmap> {
    let mut data = img.rgba.clone();
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u16;
        for c in &mut px[..3] {
            *c = ((*c as u16 * a + 127) / 255) as u8;
        }
    }
    Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(img.width, img.height)?)
}

fn fill_with_shader(
    pixmap: &mut Pixmap,
    path: &tiny_skia::Path,
    shader: Option<Shader<'static>>,
    ts: Transform,
) {
    if let Some(shader) = shader {
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        pixmap.fill_path(path, &paint, FillRule::Winding, ts, None);
    }
}

fn clip_mask(vp: &Viewport, path: &tiny_skia::Path) -> Option<Mask> {
    let mut mask = Mask::new(vp.width, vp.height)?;
    mask.fill_path(path, FillRule::Winding, true, vp.transform());
    Some(mask)
}

/// Draws `scene` into `pixmap` (already sized to `vp`).
pub fn render_into(pixmap: &mut Pixmap, scene: &Scene, vp: &Viewport) {
    let ts = vp.transform();
    for item in &scene.items {
        match item {
            Item::Board {
                outline,
                bbox,
                fill,
                stroke,
            } => {
                if let Some(f) = fill {
                    draw_board(pixmap, vp, outline, bbox, f);
                }
                if let (Some(s), Some(p)) = (stroke, to_sk_path(outline)) {
                    pixmap.stroke_path(&p, &solid(s.color), &sk_stroke(s), ts, None);
                }
            }
            Item::Path { data, fill, stroke } => {
                let Some(p) = to_sk_path(data) else { continue };
                if let Some(f) = fill {
                    pixmap.fill_path(&p, &solid(*f), FillRule::EvenOdd, ts, None);
                }
                if let Some(s) = stroke {
                    pixmap.stroke_path(&p, &solid(s.color), &sk_stroke(s), ts, None);
                }
            }
            Item::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                let Some(p) =
                    PathBuilder::from_circle(center.x as f32, center.y as f32, *radius as f32)
                else {
                    continue;
                };
                pixmap.fill_path(&p, &solid(*fill), FillRule::Winding, ts, None);
                if let Some(s) = stroke {
                    pixmap.stroke_path(&p, &solid(s.color), &sk_stroke(s), ts, None);
                }
            }
            Item::Polyline { points, stroke } => {
                if let Some(p) = to_sk_path(&PathData::polyline(points, false)) {
                    pixmap.stroke_path(&p, &solid(stroke.color), &sk_stroke(stroke), ts, None);
                }
            }
            Item::Text {
                pos,
                text,
                size,
                color: c,
                ..
            } => {
                if let Some(p) = to_sk_path(&centered_text_outline(text, *size, *pos)) {
                    pixmap.fill_path(&p, &solid(*c), FillRule::Winding, ts, None);
                }
            }
        }
    }
}

/// Renders `scene` onto a fresh pixmap filled with `background`.
pub fn render(scene: &Scene, vp: &Viewport, background: Rgba) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(vp.width, vp.height)?;
    pixmap.fill(color(background));
    render_into(&mut pixmap, scene, vp);
    Some(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{SceneOptions, build_scene};
    use sa_app::Document;
    use sa_app::document::BoardAppearance;
    use sa_app::print::PrintElements;

    fn pixel(p: &Pixmap, x: u32, y: u32) -> [u8; 4] {
        let c = p.pixel(x, y).unwrap().demultiply();
        [c.red(), c.green(), c.blue(), c.alpha()]
    }

    #[test]
    fn solid_board_renders_at_dpi() {
        let mut doc = Document::empty();
        doc.board.appearance = BoardAppearance::Solid {
            colour: "#ff0000".into(),
        };
        let scene = build_scene(&doc, &SceneOptions::export(PrintElements::default()));
        let vp = Viewport::for_scene_at_dpi(&scene, 25.4); // 10 px per cm
        assert_eq!((vp.width, vp.height), (640, 640)); // (60 + 2*2) cm
        let px = render(&scene, &vp, Rgba::WHITE).unwrap();
        assert_eq!(pixel(&px, 320, 320), [255, 0, 0, 255]);
        assert_eq!(pixel(&px, 2, 2), [255, 255, 255, 255]);
    }

    #[test]
    fn wood_board_is_clipped_to_outline() {
        let mut doc = Document::empty();
        doc.board.appearance = BoardAppearance::WoodTexture {
            preset_id: "oak".into(),
        };
        let scene = build_scene(&doc, &SceneOptions::export(PrintElements::default()));
        let vp = Viewport::for_scene_at_dpi(&scene, 25.4);
        let px = render(&scene, &vp, Rgba::WHITE).unwrap();
        let centre = pixel(&px, 320, 320);
        assert_ne!(centre, [255, 255, 255, 255]);
        assert_eq!(pixel(&px, 30, 30), [255, 255, 255, 255]); // outside circle
    }
}
