//! Text Pin Path fonts (spec 29): the web app's 20 bundled OFL families, laid
//! out with ttf-parser into flattened, positioned contours. Port of web
//! `fontCatalog.ts` + `textContours.ts`.

use sa_app::document::{FontWeight, PinPathGeometry};
use sa_domain::Point;
use sa_domain::text::{DEFAULT_SEGMENTS_PER_CURVE, GlyphCommand, commands_to_contours};
use ttf_parser::{Face, OutlineBuilder};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontCategory {
    SansSerif,
    Serif,
    Cursive,
    Monospace,
}

pub struct FontEntry {
    pub id: &'static str,
    pub family: &'static str,
    pub category: FontCategory,
    regular: &'static [u8],
    bold: Option<&'static [u8]>,
    italic: Option<&'static [u8]>,
}

impl FontEntry {
    pub fn has_bold(&self) -> bool {
        self.bold.is_some()
    }
    pub fn has_italic(&self) -> bool {
        self.italic.is_some()
    }

    /// Italic wins over weight (no Bold+Italic files); falls back to Regular.
    pub fn file(&self, weight: FontWeight, italic: bool) -> &'static [u8] {
        if italic && let Some(i) = self.italic {
            return i;
        }
        if weight == FontWeight::Bold
            && let Some(b) = self.bold
        {
            return b;
        }
        self.regular
    }
}

macro_rules! f {
    ($id:literal, $file:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/text/",
            $id,
            "/",
            $file,
            ".ttf"
        ))
    };
}

macro_rules! font {
    ($id:literal, $family:literal, $cat:ident, bold, italic) => {
        FontEntry {
            id: $id,
            family: $family,
            category: FontCategory::$cat,
            regular: f!($id, "Regular"),
            bold: Some(f!($id, "Bold")),
            italic: Some(f!($id, "Italic")),
        }
    };
    ($id:literal, $family:literal, $cat:ident, bold) => {
        FontEntry {
            id: $id,
            family: $family,
            category: FontCategory::$cat,
            regular: f!($id, "Regular"),
            bold: Some(f!($id, "Bold")),
            italic: None,
        }
    };
    ($id:literal, $family:literal, $cat:ident) => {
        FontEntry {
            id: $id,
            family: $family,
            category: FontCategory::$cat,
            regular: f!($id, "Regular"),
            bold: None,
            italic: None,
        }
    };
}

pub static FONT_CATALOG: [FontEntry; 20] = [
    font!("lato", "Lato", SansSerif, bold, italic),
    font!("poppins", "Poppins", SansSerif, bold, italic),
    font!("pt-sans", "PT Sans", SansSerif, bold, italic),
    font!("barlow", "Barlow", SansSerif, bold, italic),
    font!("hind", "Hind", SansSerif, bold),
    font!("titillium-web", "Titillium Web", SansSerif, bold, italic),
    font!("pt-serif", "PT Serif", Serif, bold, italic),
    font!("crimson-text", "Crimson Text", Serif, bold, italic),
    font!("arvo", "Arvo", Serif, bold, italic),
    font!("cardo", "Cardo", Serif, bold, italic),
    font!("tinos", "Tinos", Serif, bold, italic),
    font!("pacifico", "Pacifico", Cursive),
    font!("sacramento", "Sacramento", Cursive),
    font!("great-vibes", "Great Vibes", Cursive),
    font!("kaushan-script", "Kaushan Script", Cursive),
    font!("alex-brush", "Alex Brush", Cursive),
    font!("courier-prime", "Courier Prime", Monospace, bold, italic),
    font!("space-mono", "Space Mono", Monospace, bold, italic),
    font!("ibm-plex-mono", "IBM Plex Mono", Monospace, bold, italic),
    font!("anonymous-pro", "Anonymous Pro", Monospace, bold, italic),
];

pub const DEFAULT_FONT_ID: &str = "pt-sans";
/// Initial size (cm) of a newly placed Text Pin Path.
pub const DEFAULT_TEXT_SIZE: f64 = 5.0;

pub fn font_entry(id: &str) -> Option<&'static FontEntry> {
    FONT_CATALOG.iter().find(|f| f.id == id)
}

struct Collector {
    cmds: Vec<GlyphCommand>,
    scale: f64,
    x: f64,
}

impl Collector {
    /// Font units (y-up) → document (y-down), like opentype.js `getPath`.
    fn p(&self, x: f32, y: f32) -> Point {
        Point::new(self.x + x as f64 * self.scale, -(y as f64) * self.scale)
    }
}

impl OutlineBuilder for Collector {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.cmds.push(GlyphCommand::MoveTo(p));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.cmds.push(GlyphCommand::LineTo(p));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b) = (self.p(x1, y1), self.p(x, y));
        self.cmds.push(GlyphCommand::QuadTo(a, b));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b, c) = (self.p(x1, y1), self.p(x2, y2), self.p(x, y));
        self.cmds.push(GlyphCommand::CubicTo(a, b, c));
    }
    fn close(&mut self) {
        self.cmds.push(GlyphCommand::Close);
    }
}

/// Lays `text` out one character at a time (each letter advances by its own
/// width + `letter_spacing`), returning contours relative to the origin.
pub fn text_contours(font: &[u8], text: &str, size: f64, letter_spacing: f64) -> Vec<Vec<Point>> {
    let Ok(face) = Face::parse(font, 0) else {
        return Vec::new();
    };
    let scale = size / face.units_per_em() as f64;
    let mut contours = Vec::new();
    let mut x = 0.0;
    for ch in text.chars() {
        let Some(g) = face.glyph_index(ch) else {
            x += size * 0.5 + letter_spacing;
            continue;
        };
        let mut c = Collector {
            cmds: Vec::new(),
            scale,
            x,
        };
        face.outline_glyph(g, &mut c);
        contours.extend(commands_to_contours(&c.cmds, DEFAULT_SEGMENTS_PER_CURVE));
        x += face.glyph_hor_advance(g).unwrap_or(0) as f64 * scale + letter_spacing;
    }
    contours
}

/// Web `buildTextGeometry`: resolves the font and bakes absolute contours.
/// Always resets rotation to 0 (manual rotate/scale is discarded on regenerate).
pub fn build_text_geometry(
    origin: Point,
    text: &str,
    font_id: &str,
    weight: FontWeight,
    italic: bool,
    size: f64,
    letter_spacing: f64,
) -> PinPathGeometry {
    let entry = font_entry(font_id)
        .or_else(|| font_entry(DEFAULT_FONT_ID))
        .expect("default font");
    let contours = text_contours(entry.file(weight, italic), text, size, letter_spacing)
        .into_iter()
        .map(|c| {
            c.into_iter()
                .map(|p| Point::new(p.x + origin.x, p.y + origin.y))
                .collect()
        })
        .collect();
    PinPathGeometry::Text {
        origin,
        text: text.to_owned(),
        font_id: entry.id.to_owned(),
        weight,
        italic,
        size,
        letter_spacing,
        rotation: 0.0,
        contours,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_font_parses() {
        for f in &FONT_CATALOG {
            for bytes in [Some(f.regular), f.bold, f.italic].into_iter().flatten() {
                assert!(Face::parse(bytes, 0).is_ok(), "{}", f.id);
            }
        }
    }

    #[test]
    fn letter_o_has_two_contours_above_baseline() {
        let c = text_contours(font_entry("pt-sans").unwrap().regular, "o", 5.0, 0.0);
        assert_eq!(c.len(), 2);
        // y-down: the glyph body sits above the baseline (small overshoot allowed)
        assert!(c.iter().flatten().all(|p| p.y <= 0.15 && p.y > -5.0));
        let mean = c.iter().flatten().map(|p| p.y).sum::<f64>() / c.iter().flatten().count() as f64;
        assert!(mean < -0.5);
    }

    #[test]
    fn letter_spacing_advances() {
        let f = font_entry("pt-sans").unwrap().regular;
        let a = text_contours(f, "ii", 5.0, 0.0);
        let b = text_contours(f, "ii", 5.0, 2.0);
        let max_x = |c: &Vec<Vec<Point>>| c.iter().flatten().map(|p| p.x).fold(f64::MIN, f64::max);
        assert!((max_x(&b) - max_x(&a) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn geometry_is_offset_by_origin() {
        let g = build_text_geometry(
            Point::new(10.0, 20.0),
            "A",
            "unknown",
            FontWeight::Bold,
            false,
            5.0,
            0.0,
        );
        let PinPathGeometry::Text {
            contours, font_id, ..
        } = g
        else {
            panic!()
        };
        assert_eq!(font_id, DEFAULT_FONT_ID);
        assert!(
            contours
                .iter()
                .flatten()
                .all(|p| p.x >= 9.9 && p.y <= 20.01)
        );
    }
}
