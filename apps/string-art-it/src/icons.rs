//! Icons identical to the web app: lucide-static v1.48.0 SVGs (the exact
//! glyphs `lucide-react@1.48.0` renders) plus the Layers panel's custom
//! eye/lock glyphs. Rasterized on demand with resvg as white-on-transparent
//! textures and tinted at draw time, mirroring `stroke="currentColor"`.

use std::collections::HashMap;

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};

macro_rules! icons {
    ($($variant:ident => $path:literal),* $(,)?) => {
        /// Every icon the UI uses. Variant names follow the lucide-react
        /// component names imported by the web app.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Icon { $($variant),* }

        impl Icon {
            pub const ALL: &'static [Icon] = &[$(Icon::$variant),*];

            fn svg(self) -> &'static [u8] {
                match self {
                    $(Icon::$variant => include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"), "/../../assets/icons/", $path, ".svg"
                    ))),*
                }
            }
        }
    };
}

icons! {
    ArrowLeft => "lucide/arrow-left",
    ArrowRight => "lucide/arrow-right",
    ArrowUpToLine => "lucide/arrow-up-to-line",
    BarChart3 => "lucide/chart-column",
    Bot => "lucide/bot",
    Check => "lucide/check",
    ChevronDown => "lucide/chevron-down",
    ChevronLeft => "lucide/chevron-left",
    ChevronRight => "lucide/chevron-right",
    ChevronUp => "lucide/chevron-up",
    Circle => "lucide/circle",
    Copy => "lucide/copy",
    Croissant => "lucide/croissant",
    Download => "lucide/download",
    Ellipse => "lucide/ellipse",
    Eraser => "lucide/eraser",
    FileImage => "lucide/file-image",
    FilePlus => "lucide/file-plus",
    FileType => "lucide/file-type",
    FolderOpen => "lucide/folder-open",
    Galaxy => "lucide/galaxy",
    GitMerge => "lucide/git-merge",
    Globe => "lucide/globe",
    Grid3x3 => "lucide/grid-3x3",
    Hand => "lucide/hand",
    HelpCircle => "lucide/circle-question-mark",
    Info => "lucide/info",
    Keyboard => "lucide/keyboard",
    Layers => "lucide/layers",
    LoaderPinwheel => "lucide/loader-pinwheel",
    Magnet => "lucide/magnet",
    Maximize => "lucide/maximize",
    Minus => "lucide/minus",
    MousePointer2 => "lucide/mouse-pointer-2",
    Move => "lucide/move",
    Pause => "lucide/pause",
    PenLine => "lucide/pen-line",
    PenTool => "lucide/pen-tool",
    Pin => "lucide/pin",
    Play => "lucide/play",
    Plus => "lucide/plus",
    Printer => "lucide/printer",
    RectangleHorizontal => "lucide/rectangle-horizontal",
    Redo2 => "lucide/redo-2",
    RotateCw => "lucide/rotate-cw",
    Save => "lucide/save",
    Scaling => "lucide/scaling",
    Scissors => "lucide/scissors",
    Send => "lucide/send",
    Settings => "lucide/settings",
    Shapes => "lucide/shapes",
    SkipBack => "lucide/skip-back",
    SkipForward => "lucide/skip-forward",
    Sparkles => "lucide/sparkles",
    Spline => "lucide/spline",
    Square => "lucide/square",
    Trash2 => "lucide/trash",
    Type => "lucide/type",
    Undo2 => "lucide/undo-2",
    Video => "lucide/video",
    Wallpaper => "lucide/wallpaper",
    Wand2 => "lucide/wand-sparkles",
    Waypoints => "lucide/waypoints",
    X => "lucide/x",
    ZoomIn => "lucide/zoom-in",
    ZoomOut => "lucide/zoom-out",
    Eye => "custom/eye",
    EyeOff => "custom/eye-off",
    Lock => "custom/lock",
    Unlock => "custom/unlock",
    AppLogo => "../app-icon/favicon",
}

/// Rasterizes an SVG at `px` square pixels. `stroke_width` overrides the
/// lucide default of 2 (the web passes `strokeWidth={1.5}` in places).
/// `currentColor` becomes white so the texture can be tinted.
pub fn rasterize(svg: &[u8], px: u32, stroke_width: Option<f32>) -> Option<ColorImage> {
    let mut text = String::from_utf8_lossy(svg).replace("currentColor", "#ffffff");
    if let Some(w) = stroke_width {
        text = text.replace("stroke-width=\"2\"", &format!("stroke-width=\"{w}\""));
    }
    let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(px, px)?;
    let size = tree.size();
    let scale = px as f32 / size.width().max(size.height());
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(ColorImage::from_rgba_premultiplied(
        [px as usize, px as usize],
        pixmap.data(),
    ))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    icon: Icon,
    px: u32,
    stroke_milli: u32,
}

/// Texture cache keyed by icon, physical pixel size and stroke width.
#[derive(Default)]
pub struct IconCache {
    textures: HashMap<Key, TextureHandle>,
}

impl IconCache {
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        icon: Icon,
        size: f32,
        stroke_width: Option<f32>,
    ) -> TextureHandle {
        let px = (size * ctx.pixels_per_point()).ceil().max(1.0) as u32;
        let key = Key {
            icon,
            px,
            stroke_milli: (stroke_width.unwrap_or(2.0) * 1000.0) as u32,
        };
        self.textures
            .entry(key)
            .or_insert_with(|| {
                let image = rasterize(icon.svg(), px, stroke_width)
                    .unwrap_or_else(|| ColorImage::new([1, 1], vec![Color32::TRANSPARENT]));
                ctx.load_texture(format!("icon-{icon:?}-{px}"), image, TextureOptions::LINEAR)
            })
            .clone()
    }

    /// An `egui::Image` for `icon` at `size` points, tinted `color`.
    pub fn image(
        &mut self,
        ctx: &egui::Context,
        icon: Icon,
        size: f32,
        color: Color32,
    ) -> egui::Image<'static> {
        let tex = self.texture(ctx, icon, size, None);
        egui::Image::new((tex.id(), egui::vec2(size, size))).tint(color)
    }
}

/// Window icon from the web favicon.
pub fn app_icon() -> egui::IconData {
    let image = rasterize(Icon::AppLogo.svg(), 256, None).expect("favicon rasterizes");
    egui::IconData {
        rgba: image
            .pixels
            .iter()
            .flat_map(|c| c.to_srgba_unmultiplied())
            .collect(),
        width: 256,
        height: 256,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_rasterizes_with_visible_pixels() {
        for &icon in Icon::ALL {
            let img = rasterize(icon.svg(), 32, None).unwrap_or_else(|| panic!("{icon:?}"));
            assert!(
                img.pixels.iter().any(|p| p.a() > 0),
                "{icon:?} rendered empty"
            );
        }
    }
}
