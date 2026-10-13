//! PDF backend (spec 15): true vectors via krilla — paths, gradients, images
//! and glyph-outline labels drawn straight from the [`Scene`]. Also used for
//! multi-page print output (spec 14).

use krilla::Document;
use krilla::color::rgb;
use krilla::geom::{Path as KPath, PathBuilder, Size, Transform as KTransform};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{
    Fill, FillRule, LinearGradient, RadialGradient, SpreadMethod, Stop as KStop, Stroke as KStroke,
    StrokeDash,
};
use krilla::surface::Surface;

use sa_domain::transforms::BoundingBox;

use crate::color::Rgba;
use crate::path::{PathCmd, PathData};
use crate::scene::{BoardFill, Item, Scene, Stop, Stroke};
use crate::text::centered_text_outline;

pub const PT_PER_CM: f64 = 72.0 / 2.54;

/// 2×3 affine in krilla row order (sx, ky, kx, sy, tx, ty).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine(pub [f64; 6]);

impl Affine {
    pub const IDENTITY: Affine = Affine([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    pub fn translate(x: f64, y: f64) -> Self {
        Affine([1.0, 0.0, 0.0, 1.0, x, y])
    }

    pub fn scale(s: f64) -> Self {
        Affine([s, 0.0, 0.0, s, 0.0, 0.0])
    }

    pub fn scale_xy(sx: f64, sy: f64) -> Self {
        Affine([sx, 0.0, 0.0, sy, 0.0, 0.0])
    }

    pub fn rotate_deg(deg: f64) -> Self {
        let (s, c) = deg.to_radians().sin_cos();
        Affine([c, s, -s, c, 0.0, 0.0])
    }

    /// `self ∘ other` (apply `other` first).
    pub fn then_apply(self, other: Affine) -> Affine {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = other.0;
        Affine([
            a * a2 + c * b2,
            b * a2 + d * b2,
            a * c2 + c * d2,
            b * c2 + d * d2,
            a * e2 + c * f2 + e,
            b * e2 + d * f2 + f,
        ])
    }

    fn k(self) -> KTransform {
        let [a, b, c, d, e, f] = self.0.map(|v| v as f32);
        KTransform::from_row(a, b, c, d, e, f)
    }
}

fn kpath(d: &PathData) -> Option<KPath> {
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

fn color(c: Rgba) -> rgb::Color {
    rgb::Color::new(c.r, c.g, c.b)
}

fn opacity(c: Rgba) -> NormalizedF32 {
    NormalizedF32::new(c.opacity() as f32).unwrap_or(NormalizedF32::ONE)
}

fn solid_fill(c: Rgba, rule: FillRule) -> Fill {
    Fill {
        paint: color(c).into(),
        opacity: opacity(c),
        rule,
    }
}

fn kstroke(s: &Stroke) -> KStroke {
    KStroke {
        paint: color(s.color).into(),
        width: s.width as f32,
        opacity: opacity(s.color),
        dash: s.dash.as_ref().map(|d| {
            let mut array: Vec<f32> = d.array.iter().map(|v| *v as f32).collect();
            if array.len() % 2 == 1 {
                array.extend_from_within(..);
            }
            StrokeDash {
                array,
                offset: d.offset as f32,
            }
        }),
        ..KStroke::default()
    }
}

fn stops(s: &[Stop]) -> Vec<KStop> {
    s.iter()
        .map(|s| KStop {
            offset: NormalizedF32::new(s.offset as f32).unwrap_or(NormalizedF32::ZERO),
            color: color(s.color).into(),
            opacity: opacity(s.color),
        })
        .collect()
}

fn bbox_affine(b: &BoundingBox) -> Affine {
    Affine([b.width(), 0.0, 0.0, b.height(), b.min_x, b.min_y])
}

fn draw_board(surface: &mut Surface, outline: &KPath, bbox: &BoundingBox, fill: &BoardFill) {
    match fill {
        BoardFill::Solid(c) => {
            surface.set_fill(Some(solid_fill(*c, FillRule::NonZero)));
            surface.set_stroke(None);
            surface.draw_path(outline);
        }
        BoardFill::Linear {
            stops: s,
            direction_deg,
        } => {
            let g = LinearGradient {
                x1: 0.0,
                y1: 0.0,
                x2: 1.0,
                y2: 0.0,
                transform: bbox_affine(bbox)
                    .then_apply(Affine::rotate_deg(*direction_deg))
                    .k(),
                spread_method: SpreadMethod::Pad,
                stops: stops(s),
                anti_alias: true,
            };
            surface.set_fill(Some(Fill {
                paint: g.into(),
                opacity: NormalizedF32::ONE,
                rule: FillRule::NonZero,
            }));
            surface.set_stroke(None);
            surface.draw_path(outline);
        }
        BoardFill::Radial { stops: s, centre } => {
            let (cx, cy) = (centre.x as f32, centre.y as f32);
            let g = RadialGradient {
                fx: cx,
                fy: cy,
                fr: 0.0,
                cx,
                cy,
                cr: 0.5,
                transform: bbox_affine(bbox).k(),
                spread_method: SpreadMethod::Pad,
                stops: stops(s),
                anti_alias: true,
            };
            surface.set_fill(Some(Fill {
                paint: g.into(),
                opacity: NormalizedF32::ONE,
                rule: FillRule::NonZero,
            }));
            surface.set_stroke(None);
            surface.draw_path(outline);
        }
        BoardFill::Wood { bands, tint } => {
            surface.push_clip_path(outline, &FillRule::NonZero);
            surface.push_transform(&bbox_affine(bbox).k());
            surface.set_stroke(None);
            // Outermost first, as solid disks (no seams; same coverage as annuli).
            if let Some(last) = bands.last()
                && let Some(r) = kpath(&PathData::polyline(
                    &[
                        sa_domain::Point::new(0.0, 0.0),
                        sa_domain::Point::new(1.0, 0.0),
                        sa_domain::Point::new(1.0, 1.0),
                        sa_domain::Point::new(0.0, 1.0),
                    ],
                    true,
                ))
            {
                surface.set_fill(Some(solid_fill(last.colour, FillRule::NonZero)));
                surface.draw_path(&r);
            }
            for band in bands.iter().rev() {
                if let Some(p) = kpath(&PathData::polyline(&band.outer, true)) {
                    surface.set_fill(Some(solid_fill(band.colour, FillRule::NonZero)));
                    surface.draw_path(&p);
                }
            }
            if let Some(t) = tint
                && let Some(r) = kpath(&PathData::polyline(
                    &[
                        sa_domain::Point::new(0.0, 0.0),
                        sa_domain::Point::new(1.0, 0.0),
                        sa_domain::Point::new(1.0, 1.0),
                        sa_domain::Point::new(0.0, 1.0),
                    ],
                    true,
                ))
            {
                surface.set_fill(Some(solid_fill(*t, FillRule::NonZero)));
                surface.draw_path(&r);
            }
            surface.pop();
            surface.pop();
        }
        BoardFill::Image(img) => {
            let image = krilla::image::Image::from_rgba8(img.rgba.clone(), img.width, img.height);
            let s = (bbox.width() / img.width as f64).max(bbox.height() / img.height as f64);
            let (w, h) = (img.width as f64 * s, img.height as f64 * s);
            let tx = bbox.min_x + (bbox.width() - w) / 2.0;
            let ty = bbox.min_y + (bbox.height() - h) / 2.0;
            surface.push_clip_path(outline, &FillRule::NonZero);
            surface.push_transform(&Affine::translate(tx, ty).k());
            if let Some(size) = Size::from_wh(w as f32, h as f32) {
                surface.draw_image(image, size);
            }
            surface.pop();
            surface.pop();
        }
    }
}

/// Draws `scene` (cm) onto `surface` under `xf` (cm → page points).
pub fn draw_scene(surface: &mut Surface, scene: &Scene, xf: Affine) {
    surface.push_transform(&xf.k());
    for item in &scene.items {
        match item {
            Item::Board {
                outline,
                bbox,
                fill,
                stroke,
            } => {
                let Some(p) = kpath(outline) else { continue };
                if let Some(f) = fill {
                    draw_board(surface, &p, bbox, f);
                }
                if let Some(s) = stroke {
                    surface.set_fill(None);
                    surface.set_stroke(Some(kstroke(s)));
                    surface.draw_path(&p);
                }
            }
            Item::Path { data, fill, stroke } => {
                let Some(p) = kpath(data) else { continue };
                surface.set_fill(fill.map(|c| solid_fill(c, FillRule::EvenOdd)));
                surface.set_stroke(stroke.as_ref().map(kstroke));
                surface.draw_path(&p);
            }
            Item::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                let circle =
                    PathData::from_paths([&sa_domain::shapes::circle_shape(*center, *radius)]);
                let Some(p) = kpath(&circle) else { continue };
                surface.set_fill(Some(solid_fill(*fill, FillRule::NonZero)));
                surface.set_stroke(stroke.as_ref().map(kstroke));
                surface.draw_path(&p);
            }
            Item::Polyline { points, stroke } => {
                let Some(p) = kpath(&PathData::polyline(points, false)) else {
                    continue;
                };
                surface.set_fill(None);
                surface.set_stroke(Some(kstroke(stroke)));
                surface.draw_path(&p);
            }
            Item::Text {
                pos,
                text,
                size,
                color: c,
                ..
            } => {
                let Some(p) = kpath(&centered_text_outline(text, *size, *pos)) else {
                    continue;
                };
                surface.set_fill(Some(solid_fill(*c, FillRule::NonZero)));
                surface.set_stroke(None);
                surface.draw_path(&p);
            }
        }
    }
    surface.set_fill(None);
    surface.set_stroke(None);
    surface.pop();
}

/// Multi-page PDF builder; page sizes in centimetres.
pub struct PdfBuilder {
    doc: Document,
}

impl Default for PdfBuilder {
    fn default() -> Self {
        Self {
            doc: Document::new(),
        }
    }
}

impl PdfBuilder {
    /// Adds a page; `draw` receives the surface and the cm → pt transform.
    pub fn page(&mut self, width_cm: f64, height_cm: f64, draw: impl FnOnce(&mut Surface, Affine)) {
        let settings = PageSettings::from_wh(
            (width_cm * PT_PER_CM) as f32,
            (height_cm * PT_PER_CM) as f32,
        )
        .unwrap_or_else(|| PageSettings::from_wh(1.0, 1.0).expect("1×1 page"));
        let mut page = self.doc.start_page_with(settings);
        let mut surface = page.surface();
        draw(&mut surface, Affine::scale(PT_PER_CM));
        surface.finish();
        page.finish();
    }

    pub fn finish(self) -> Result<Vec<u8>, String> {
        self.doc.finish().map_err(|e| format!("{e:?}"))
    }
}

/// Single-page vector export at true physical size (spec 15).
pub fn scene_to_pdf(scene: &Scene) -> Result<Vec<u8>, String> {
    let b = scene.bounds;
    let mut pdf = PdfBuilder::default();
    pdf.page(b.width(), b.height(), |s, xf| {
        draw_scene(
            s,
            scene,
            xf.then_apply(Affine::translate(-b.min_x, -b.min_y)),
        );
    });
    pdf.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{SceneOptions, build_scene};
    use sa_app::Document as SaDocument;
    use sa_app::document::BoardAppearance;
    use sa_app::print::PrintElements;

    #[test]
    fn exports_valid_pdf_for_every_fill() {
        for a in [
            BoardAppearance::Solid {
                colour: "#123456".into(),
            },
            BoardAppearance::WoodTexture {
                preset_id: "oak".into(),
            },
            BoardAppearance::PaintedWood {
                preset_id: "sage".into(),
            },
            BoardAppearance::LinearGradient {
                stops: vec![
                    sa_app::document::GradientStop {
                        offset: 0.0,
                        colour: "#000".into(),
                    },
                    sa_app::document::GradientStop {
                        offset: 100.0,
                        colour: "#fff".into(),
                    },
                ],
                direction: 45.0,
            },
        ] {
            let mut doc = SaDocument::empty();
            doc.board.appearance = a;
            let scene = build_scene(&doc, &SceneOptions::export(PrintElements::default()));
            let bytes = scene_to_pdf(&scene).unwrap();
            assert!(bytes.starts_with(b"%PDF-"));
            // 64 cm page → 1814.17 pt
            let text = String::from_utf8_lossy(&bytes);
            assert!(text.contains("MediaBox"), "has page box");
        }
    }

    #[test]
    fn affine_compose() {
        let a = Affine::translate(10.0, 0.0).then_apply(Affine::scale(2.0));
        assert_eq!(a.0, [2.0, 0.0, 0.0, 2.0, 10.0, 0.0]);
    }
}
