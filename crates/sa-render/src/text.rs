//! Pin-number labels rendered as glyph outlines, so raster and PDF output look
//! the same without depending on system fonts. Uses PT Sans (bundled, OFL).

use std::sync::OnceLock;

use sa_domain::Point;
use ttf_parser::{Face, OutlineBuilder};

use crate::path::{PathCmd, PathData};

static LABEL_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/text/pt-sans/Regular.ttf"
));

fn face() -> &'static Face<'static> {
    static FACE: OnceLock<Face<'static>> = OnceLock::new();
    FACE.get_or_init(|| Face::parse(LABEL_FONT, 0).expect("bundled label font parses"))
}

struct Builder<'a> {
    out: &'a mut PathData,
    scale: f64,
    origin: Point,
    last: Point,
}

impl Builder<'_> {
    fn map(&self, x: f32, y: f32) -> Point {
        // Font units are y-up; documents are y-down.
        Point::new(
            self.origin.x + x as f64 * self.scale,
            self.origin.y - y as f64 * self.scale,
        )
    }
}

impl OutlineBuilder for Builder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.last = self.map(x, y);
        self.out.cmds.push(PathCmd::MoveTo(self.last));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.last = self.map(x, y);
        self.out.cmds.push(PathCmd::LineTo(self.last));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (p0, q, p) = (self.last, self.map(x1, y1), self.map(x, y));
        let c1 = Point::new(
            p0.x + 2.0 / 3.0 * (q.x - p0.x),
            p0.y + 2.0 / 3.0 * (q.y - p0.y),
        );
        let c2 = Point::new(p.x + 2.0 / 3.0 * (q.x - p.x), p.y + 2.0 / 3.0 * (q.y - p.y));
        self.out.cmds.push(PathCmd::CubicTo(c1, c2, p));
        self.last = p;
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, p) = (self.map(x1, y1), self.map(x2, y2), self.map(x, y));
        self.out.cmds.push(PathCmd::CubicTo(c1, c2, p));
        self.last = p;
    }
    fn close(&mut self) {
        self.out.cmds.push(PathCmd::Close);
    }
}

/// Outlines `text` at `size` (cm, em size) centred on `center` like SVG
/// `text-anchor="middle" dominant-baseline="middle"`.
pub fn centered_text_outline(text: &str, size: f64, center: Point) -> PathData {
    let face = face();
    let scale = size / face.units_per_em() as f64;
    let glyphs: Vec<_> = text.chars().filter_map(|c| face.glyph_index(c)).collect();
    let width: f64 = glyphs
        .iter()
        .map(|g| face.glyph_hor_advance(*g).unwrap_or(0) as f64 * scale)
        .sum();
    let x_height = face.x_height().unwrap_or((face.units_per_em() / 2) as i16) as f64 * scale;
    let mut out = PathData::new();
    let mut pen = Point::new(center.x - width / 2.0, center.y + x_height / 2.0);
    for g in glyphs {
        let mut b = Builder {
            out: &mut out,
            scale,
            origin: pen,
            last: pen,
        };
        face.outline_glyph(g, &mut b);
        pen.x += face.glyph_hor_advance(g).unwrap_or(0) as f64 * scale;
    }
    out
}
