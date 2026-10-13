//! Printing (spec 14): page layout (single page or overlapping tiles),
//! multi-page vector PDF, raster page previews, calibration page, and
//! dispatch to a system printer (CUPS / winspool via `printers`) or the OS
//! PDF viewer. Replaces the web app's `window.print()` + CSS `@page`.

use sa_app::Document;
use sa_app::print::{
    CALIBRATION_LENGTH_CM, PRINT_MARGIN_CM, PrintSettings, tile_coordinate, tile_grid,
};
use sa_domain::Point;
use sa_domain::transforms::BoundingBox;
use sa_render::Rgba;
use sa_render::pdf::{Affine, PdfBuilder, draw_scene};
use sa_render::raster::{self, Viewport};
use sa_render::scene::{Item, Scene, SceneOptions, Stroke, TextAnchor, board_bbox, build_scene};

const PX_PER_CM: f64 = 96.0 / 2.54;

#[derive(Clone, Debug, PartialEq)]
pub struct PrintPage {
    /// Where the board bbox's top-left lands on the paper (cm).
    pub offset: Point,
    pub label: Option<String>,
    pub trim_marks: bool,
    pub alignment_marks: bool,
}

#[derive(Clone, Debug)]
pub struct PrintLayout {
    pub paper: (f64, f64),
    pub scale: f64,
    pub bbox: BoundingBox,
    pub cols: usize,
    pub rows: usize,
    pub pages: Vec<PrintPage>,
    pub scene: Scene,
}

/// Lays out the print job. Tiling only engages when enabled AND the printed
/// board exceeds one page's content area.
pub fn layout(doc: &Document, s: &PrintSettings, grid_gap: (f64, f64)) -> PrintLayout {
    let bbox = board_bbox(doc);
    let scale = s.effective_scale(bbox.width(), bbox.height());
    let paper = s.paper_cm();
    let printed = (bbox.width() * scale, bbox.height() * scale);
    let area = (
        paper.0 - PRINT_MARGIN_CM * 2.0,
        paper.1 - PRINT_MARGIN_CM * 2.0,
    );
    let scene = build_scene(doc, &SceneOptions::print(s.elements, scale, grid_gap));
    let t = &s.tiling;
    if t.enabled && (printed.0 > area.0 || printed.1 > area.1) {
        let (cols, rows, tiles) = tile_grid(printed, area, t.overlap_cm);
        let pages = tiles
            .iter()
            .map(|tile| {
                let parts: Vec<String> = [
                    t.page_coordinates.then(|| tile_coordinate(tile)),
                    t.page_numbers
                        .then(|| (tile.row * cols + tile.col + 1).to_string()),
                ]
                .into_iter()
                .flatten()
                .collect();
                PrintPage {
                    offset: Point::new(PRINT_MARGIN_CM - tile.x, PRINT_MARGIN_CM - tile.y),
                    label: (!parts.is_empty()).then(|| parts.join(" · ")),
                    trim_marks: t.trim_marks,
                    alignment_marks: t.alignment_marks,
                }
            })
            .collect();
        return PrintLayout {
            paper,
            scale,
            bbox,
            cols,
            rows,
            pages,
            scene,
        };
    }
    let page = PrintPage {
        offset: Point::new((paper.0 - printed.0) / 2.0, (paper.1 - printed.1) / 2.0),
        label: None,
        trim_marks: false,
        alignment_marks: false,
    };
    PrintLayout {
        paper,
        scale,
        bbox,
        cols: 1,
        rows: 1,
        pages: vec![page],
        scene,
    }
}

/// Trim marks, alignment cross and page label, in paper cm.
fn marks(paper: (f64, f64), page: &PrintPage) -> Scene {
    let px = |v: f64| v / PX_PER_CM;
    let (w, h) = paper;
    let mut items = Vec::new();
    let line = |a: Point, b: Point, alpha: f64| Item::Polyline {
        points: vec![a, b],
        stroke: Stroke::solid(Rgba::BLACK.with_opacity(alpha), px(1.0)),
    };
    if page.trim_marks {
        let m = px(12.0);
        for (cx, cy) in [(m, m), (w - m, m), (m, h - m), (w - m, h - m)] {
            let x0 = if cx < w / 2.0 { 0.0 } else { w };
            let y0 = if cy < h / 2.0 { 0.0 } else { h };
            items.push(line(Point::new(x0, cy), Point::new(cx, cy), 0.6));
            items.push(line(Point::new(cx, y0), Point::new(cx, cy), 0.6));
        }
    }
    if page.alignment_marks {
        let (cx, cy, d) = (w / 2.0, h / 2.0, px(6.0));
        items.push(line(Point::new(cx - d, cy), Point::new(cx + d, cy), 0.5));
        items.push(line(Point::new(cx, cy - d), Point::new(cx, cy + d), 0.5));
    }
    if let Some(label) = &page.label {
        let size = px(11.0);
        let approx_w = label.chars().count() as f64 * size * 0.55;
        items.push(Item::Text {
            pos: Point::new(w - px(8.0) - approx_w / 2.0, h - px(8.0) - size * 0.35),
            text: label.clone(),
            size,
            color: Rgba::BLACK.with_opacity(0.7),
            anchor: TextAnchor::Middle,
        });
    }
    Scene {
        bounds: BoundingBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: w,
            max_y: h,
        },
        items,
    }
}

/// Paper-cm transform for a page: offset + (doc − bbox.min) × scale.
fn page_affine(l: &PrintLayout, page: &PrintPage) -> Affine {
    Affine::translate(page.offset.x, page.offset.y)
        .then_apply(Affine::scale(l.scale))
        .then_apply(Affine::translate(-l.bbox.min_x, -l.bbox.min_y))
}

pub fn layout_pdf(l: &PrintLayout) -> Result<Vec<u8>, String> {
    let mut pdf = PdfBuilder::default();
    for page in &l.pages {
        let m = marks(l.paper, page);
        pdf.page(l.paper.0, l.paper.1, |s, xf| {
            draw_scene(s, &l.scene, xf.then_apply(page_affine(l, page)));
            draw_scene(s, &m, xf);
        });
    }
    pdf.finish()
}

/// The 10 cm calibration reference page (spec 14).
pub fn calibration_pdf(s: &PrintSettings) -> Result<Vec<u8>, String> {
    let (w, h) = s.paper_cm();
    let (y, x0) = (h / 2.0, (w - CALIBRATION_LENGTH_CM) / 2.0);
    let x1 = x0 + CALIBRATION_LENGTH_CM;
    let st = Stroke::solid(Rgba::BLACK, 1.5 / PX_PER_CM);
    let tick = 8.0 / PX_PER_CM;
    let scene = Scene {
        bounds: BoundingBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: w,
            max_y: h,
        },
        items: vec![
            Item::Polyline {
                points: vec![Point::new(x0, y), Point::new(x1, y)],
                stroke: st.clone(),
            },
            Item::Polyline {
                points: vec![Point::new(x0, y - tick), Point::new(x0, y + tick)],
                stroke: st.clone(),
            },
            Item::Polyline {
                points: vec![Point::new(x1, y - tick), Point::new(x1, y + tick)],
                stroke: st,
            },
            Item::Text {
                pos: Point::new(w / 2.0, y - 16.0 / PX_PER_CM),
                text: format!("{CALIBRATION_LENGTH_CM} cm"),
                size: 14.0 / PX_PER_CM,
                color: Rgba::BLACK,
                anchor: TextAnchor::Middle,
            },
        ],
    };
    let mut pdf = PdfBuilder::default();
    pdf.page(w, h, |s, xf| draw_scene(s, &scene, xf));
    pdf.finish()
}

/// RGBA preview of one page at `px_per_cm` (white paper).
pub fn render_page_preview(
    l: &PrintLayout,
    page: &PrintPage,
    px_per_cm: f64,
) -> Option<(u32, u32, Vec<u8>)> {
    let paper_vp = Viewport::for_bounds(
        &BoundingBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: l.paper.0,
            max_y: l.paper.1,
        },
        px_per_cm,
    );
    let mut pixmap = raster::render(
        &Scene {
            bounds: paper_vp_bounds(l),
            items: vec![],
        },
        &paper_vp,
        Rgba::WHITE,
    )?;
    // Document space: px = (doc − origin) × scale·px_per_cm with origin chosen so
    // that bbox.min lands at page.offset.
    let s = l.scale * px_per_cm;
    let doc_vp = Viewport {
        origin: Point::new(
            l.bbox.min_x - page.offset.x / l.scale,
            l.bbox.min_y - page.offset.y / l.scale,
        ),
        scale: s,
        width: paper_vp.width,
        height: paper_vp.height,
    };
    raster::render_into(&mut pixmap, &l.scene, &doc_vp);
    raster::render_into(&mut pixmap, &marks(l.paper, page), &paper_vp);
    Some((paper_vp.width, paper_vp.height, pixmap.data().to_vec()))
}

fn paper_vp_bounds(l: &PrintLayout) -> BoundingBox {
    BoundingBox {
        min_x: 0.0,
        min_y: 0.0,
        max_x: l.paper.0,
        max_y: l.paper.1,
    }
}

/// System printers for the print dialog.
pub fn printer_names() -> Vec<(String, bool)> {
    printers::get_printers()
        .into_iter()
        .map(|p| (p.name.clone(), p.is_default))
        .collect()
}

/// Sends PDF bytes to `printer` (or the default). Falls back to opening the
/// PDF in the system viewer (whose own dialog prints) when no printer exists.
pub fn print_pdf(bytes: &[u8], printer: Option<&str>, job_name: &str) -> Result<String, String> {
    let target = printer
        .and_then(printers::get_printer_by_name)
        .or_else(printers::get_default_printer);
    let path = std::env::temp_dir().join(format!("{}.pdf", job_name.replace(['/', '\\'], "_")));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    match target {
        Some(p) => {
            let opts = printers::common::base::job::PrinterJobOptions {
                name: Some(job_name),
                ..printers::common::base::job::PrinterJobOptions::none()
            };
            p.print_file(&path.to_string_lossy(), opts)
                .map_err(|e| format!("{e:?}"))?;
            Ok(p.name.clone())
        }
        None => {
            open::that(&path).map_err(|e| e.to_string())?;
            Ok(path.display().to_string())
        }
    }
}

/// Opens the PDF in the OS viewer (preview / print from there).
pub fn open_pdf(bytes: &[u8], name: &str) -> Result<(), String> {
    let path = std::env::temp_dir().join(format!("{name}.pdf"));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    open::that(&path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_app::print::ScaleMode;

    #[test]
    fn single_page_is_centred_and_fit() {
        let doc = Document::empty();
        let l = layout(&doc, &PrintSettings::default(), (1.0, 1.0));
        assert_eq!(l.pages.len(), 1);
        assert!((l.scale - 19.0 / 60.0).abs() < 1e-9);
        assert!((l.pages[0].offset.x - 1.0).abs() < 1e-9);
        assert!(layout_pdf(&l).unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn tiles_at_actual_size() {
        let doc = Document::empty();
        let mut s = PrintSettings {
            scale_mode: ScaleMode::Actual,
            ..Default::default()
        };
        s.tiling.enabled = true;
        let l = layout(&doc, &s, (1.0, 1.0));
        assert_eq!((l.cols, l.rows), (4, 3));
        assert_eq!(l.pages[5].label.as_deref(), Some("B2 · 6"));
        assert_eq!(l.pages[1].offset, Point::new(1.0 - 18.0, 1.0));
        let (w, h, px) = render_page_preview(&l, &l.pages[0], 10.0).unwrap();
        assert_eq!((w, h), (210, 297));
        assert_eq!(px.len(), (w * h * 4) as usize);
        assert!(calibration_pdf(&s).unwrap().starts_with(b"%PDF"));
    }
}
