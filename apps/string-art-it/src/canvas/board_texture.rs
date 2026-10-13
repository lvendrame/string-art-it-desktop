//! The board background rendered by the shared tiny-skia backend into an egui
//! texture, so gradients, wood grain and custom textures match exports exactly.
//! Re-rendered only when the board changes or zoom needs more resolution.

use egui::{ColorImage, TextureHandle, TextureOptions};
use sa_app::document::Board;
use sa_domain::transforms::BoundingBox;
use sa_render::raster::{self, Viewport as RasterViewport};
use sa_render::scene::{Item, Scene, board_fill};
use sa_render::{Rgba, path::PathData};

const MAX_TEXTURE_PX: f64 = 4096.0;

#[derive(Default)]
pub struct BoardTexture {
    board: Option<Board>,
    px_per_cm: f64,
    texture: Option<TextureHandle>,
    pub bbox: BoundingBox,
}

impl BoardTexture {
    /// Returns the texture for `board` at roughly `wanted_px_per_cm`.
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        board: &Board,
        wanted_px_per_cm: f64,
    ) -> Option<&TextureHandle> {
        let bbox = sa_domain::transforms::bounding_box_of(&board.path().bounding_box_points());
        let max_scale = MAX_TEXTURE_PX / bbox.width().max(bbox.height()).max(1e-6);
        let wanted = wanted_px_per_cm.min(max_scale).max(1.0);
        let stale = self.board.as_ref() != Some(board)
            || wanted > self.px_per_cm * 1.25
            || wanted < self.px_per_cm / 2.5;
        if stale {
            let fill = board_fill(&board.appearance)?;
            let scene = Scene {
                bounds: bbox,
                items: vec![Item::Board {
                    outline: PathData::from_paths([&board.path()]),
                    bbox,
                    fill: Some(fill),
                    stroke: None,
                }],
            };
            let vp = RasterViewport::for_bounds(&bbox, wanted);
            let pixmap = raster::render(&scene, &vp, Rgba::TRANSPARENT)?;
            let image = ColorImage::from_rgba_premultiplied(
                [vp.width as usize, vp.height as usize],
                pixmap.data(),
            );
            match &mut self.texture {
                Some(t) => t.set(image, TextureOptions::LINEAR),
                None => {
                    self.texture =
                        Some(ctx.load_texture("board-fill", image, TextureOptions::LINEAR))
                }
            }
            self.board = Some(board.clone());
            self.px_per_cm = wanted;
            self.bbox = bbox;
        }
        self.texture.as_ref()
    }
}
