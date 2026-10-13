//! Export (spec 15): SVG / PDF vectors and PNG / JPEG rasters at a chosen
//! DPI, all from the same export scene. Port of web `infrastructure/export`.

use std::io::Cursor;

use sa_app::Document;
use sa_app::print::PrintElements;
use sa_render::raster::{self, Viewport};
use sa_render::scene::{SceneOptions, build_scene};
use sa_render::{Rgba, pdf, svg};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Svg,
    Pdf,
    Png,
    Jpeg,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpg",
        }
    }
}

pub const DEFAULT_DPI: f64 = 150.0;
const JPEG_QUALITY: u8 = 95;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("image too large to render")]
    TooLarge,
    #[error("{0}")]
    Encode(String),
}

/// Renders `doc` in `format`. Rasters are physical size × DPI; JPEG is
/// flattened onto white, PNG keeps transparency outside the board.
pub fn export(
    doc: &Document,
    elements: PrintElements,
    format: ExportFormat,
    dpi: f64,
) -> Result<Vec<u8>, ExportError> {
    let scene = build_scene(doc, &SceneOptions::export(elements));
    match format {
        ExportFormat::Svg => Ok(svg::to_svg(&scene).into_bytes()),
        ExportFormat::Pdf => pdf::scene_to_pdf(&scene).map_err(ExportError::Encode),
        ExportFormat::Png | ExportFormat::Jpeg => {
            let vp = Viewport::for_scene_at_dpi(&scene, dpi.clamp(10.0, 1200.0));
            if vp.width as u64 * vp.height as u64 > 400_000_000 {
                return Err(ExportError::TooLarge);
            }
            let bg = if format == ExportFormat::Jpeg {
                Rgba::WHITE
            } else {
                Rgba::TRANSPARENT
            };
            let pixmap = raster::render(&scene, &vp, bg).ok_or(ExportError::TooLarge)?;
            let mut rgba = pixmap.data().to_vec();
            // tiny-skia is premultiplied; image wants straight alpha.
            for px in rgba.chunks_exact_mut(4) {
                let a = px[3] as u16;
                if a > 0 && a < 255 {
                    for c in &mut px[..3] {
                        *c = ((*c as u16 * 255 + a / 2) / a).min(255) as u8;
                    }
                }
            }
            let img = image::RgbaImage::from_raw(vp.width, vp.height, rgba)
                .ok_or(ExportError::TooLarge)?;
            let mut out = Cursor::new(Vec::new());
            if format == ExportFormat::Png {
                img.write_to(&mut out, image::ImageFormat::Png)
                    .map_err(|e| ExportError::Encode(e.to_string()))?;
            } else {
                let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
                    .encode_image(&rgb)
                    .map_err(|e| ExportError::Encode(e.to_string()))?;
            }
            Ok(out.into_inner())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_format_produces_its_magic_bytes() {
        let doc = Document::empty();
        let el = PrintElements::default();
        assert!(
            export(&doc, el, ExportFormat::Svg, 0.0)
                .unwrap()
                .starts_with(b"<?xml")
        );
        assert!(
            export(&doc, el, ExportFormat::Pdf, 0.0)
                .unwrap()
                .starts_with(b"%PDF")
        );
        let png = export(&doc, el, ExportFormat::Png, 25.4).unwrap();
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        let img = image::load_from_memory(&png).unwrap();
        assert_eq!((img.width(), img.height()), (640, 640)); // 64 cm at 10 px/cm
        assert!(
            export(&doc, el, ExportFormat::Jpeg, 25.4)
                .unwrap()
                .starts_with(&[0xFF, 0xD8])
        );
    }
}

#[cfg(test)]
mod visual {
    use super::*;
    use sa_app::document::{
        BoardAppearance, PinPath, PinPathGeometry, PinStyle, SymmetryConfig, ThreadPath,
    };
    use sa_domain::Point;

    /// `STRINGARTIT_EXPORT_DIR=… cargo test -p sa-infra visual` writes samples.
    #[test]
    fn write_samples_when_requested() {
        let Ok(dir) = std::env::var("STRINGARTIT_EXPORT_DIR") else {
            return;
        };
        let mut doc = Document::empty();
        doc.board.appearance = BoardAppearance::WoodTexture {
            preset_id: "walnut".into(),
        };
        let path = PinPath::new(
            PinPathGeometry::Circle {
                center: Point::ORIGIN,
                radius: 25.0,
            },
            3.0,
            &PinStyle {
                colour: "#f2ede4".into(),
                diameter: 2.0,
                guide_visible: true,
            },
            SymmetryConfig::None,
        );
        let n = path.pins.len();
        let ids: Vec<String> = (0..60)
            .map(|k| path.pins[(k * 17) % n].id.clone())
            .collect();
        doc.pin_layers[0].pin_paths.push(path);
        doc.thread_layers[0].thread_paths.push(ThreadPath::new(
            ids,
            vec!["#5b8def".into(), "#edeff7".into()],
            1.5,
            6.0,
        ));
        for f in [ExportFormat::Pdf, ExportFormat::Png, ExportFormat::Svg] {
            let bytes = export(&doc, PrintElements::default(), f, 60.0).unwrap();
            std::fs::write(format!("{dir}/sample.{}", f.extension()), bytes).unwrap();
        }
    }
}

#[cfg(test)]
mod generator_gallery {
    use super::*;
    use sa_app::editor::Store;
    use sa_app::generator::PatternId;

    /// Renders every generator pattern to PNG when STRINGARTIT_SNAPSHOT_DIR is set.
    #[test]
    fn all_patterns_export() {
        let dir = std::env::var_os("STRINGARTIT_SNAPSHOT_DIR");
        let colours: Vec<String> = ["#e8b449", "#d9534f", "#5b8def", "#5cb85c"]
            .map(String::from)
            .to_vec();
        for id in PatternId::ALL {
            let mut s = Store::default();
            s.generate_pattern(id.default_params(), colours.clone());
            s.confirm_generated_pattern();
            let png = export(&s.doc, PrintElements::default(), ExportFormat::Png, 40.0).unwrap();
            if let Some(d) = &dir {
                std::fs::write(
                    std::path::Path::new(d).join(format!("gen-{:02}.png", id as usize)),
                    png,
                )
                .unwrap();
            }
        }
    }
}
