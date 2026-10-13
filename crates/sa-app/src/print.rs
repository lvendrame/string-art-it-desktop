//! Print/export settings (spec 14). Port of web `printSettings.ts`.

use serde::{Deserialize, Serialize};

/// Which elements print/export — independent of editor visibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintElements {
    pub board_outline: bool,
    pub background: bool,
    pub pins: bool,
    pub pin_guides: bool,
    pub pin_numbers: bool,
    pub threads: bool,
    pub grid: bool,
}

impl Default for PrintElements {
    fn default() -> Self {
        Self {
            board_outline: true,
            background: true,
            pins: true,
            pin_guides: true,
            pin_numbers: true,
            threads: true,
            grid: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleMode {
    /// Actual size (× calibration factor).
    Actual,
    Fit,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaperSize {
    A4,
    A3,
    Letter,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tiling {
    pub enabled: bool,
    pub overlap_cm: f64,
    pub trim_marks: bool,
    pub alignment_marks: bool,
    pub page_numbers: bool,
    pub page_coordinates: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrintSettings {
    pub elements: PrintElements,
    pub scale_mode: ScaleMode,
    pub custom_ratio: f64,
    pub paper: PaperSize,
    pub custom_width_cm: f64,
    pub custom_height_cm: f64,
    pub orientation: Orientation,
    pub correction_factor: f64,
    pub tiling: Tiling,
}

impl Default for PrintSettings {
    /// Web `defaultPrintSettings`.
    fn default() -> Self {
        Self {
            elements: PrintElements {
                board_outline: true,
                background: true,
                pins: true,
                pin_guides: true,
                pin_numbers: false,
                threads: false,
                grid: false,
            },
            scale_mode: ScaleMode::Fit,
            custom_ratio: 1.0,
            paper: PaperSize::A4,
            custom_width_cm: 21.0,
            custom_height_cm: 29.7,
            orientation: Orientation::Portrait,
            correction_factor: 1.0,
            tiling: Tiling {
                enabled: false,
                overlap_cm: 1.0,
                trim_marks: true,
                alignment_marks: true,
                page_numbers: true,
                page_coordinates: true,
            },
        }
    }
}

pub const PRINT_MARGIN_CM: f64 = 1.0;
pub const CALIBRATION_LENGTH_CM: f64 = 10.0;

impl PrintSettings {
    /// Paper size in cm after orientation.
    pub fn paper_cm(&self) -> (f64, f64) {
        let (w, h) = match self.paper {
            PaperSize::A4 => (21.0, 29.7),
            PaperSize::A3 => (29.7, 42.0),
            PaperSize::Letter => (21.59, 27.94),
            PaperSize::Custom => (self.custom_width_cm, self.custom_height_cm),
        };
        match self.orientation {
            Orientation::Portrait => (w, h),
            Orientation::Landscape => (h, w),
        }
    }

    /// Board-cm → paper-cm multiplier; calibration applies to 1:1 only.
    pub fn effective_scale(&self, board_w: f64, board_h: f64) -> f64 {
        match self.scale_mode {
            ScaleMode::Actual => self.correction_factor,
            ScaleMode::Custom => self.custom_ratio,
            ScaleMode::Fit => {
                let (pw, ph) = self.paper_cm();
                let aw = (pw - PRINT_MARGIN_CM * 2.0).max(0.1);
                let ah = (ph - PRINT_MARGIN_CM * 2.0).max(0.1);
                (aw / board_w.max(1e-9)).min(ah / board_h.max(1e-9))
            }
        }
    }
}

/// `requested / measured` (spec 14 calibration).
pub fn correction_factor(requested_cm: f64, measured_cm: f64) -> f64 {
    if measured_cm <= 0.0 {
        1.0
    } else {
        requested_cm / measured_cm
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tile {
    pub col: usize,
    pub row: usize,
    /// Top-left in printed-content space (cm).
    pub x: f64,
    pub y: f64,
}

/// Overlapping tile grid: each tile starts `tile − overlap` after the previous.
pub fn tile_grid(content: (f64, f64), tile: (f64, f64), overlap: f64) -> (usize, usize, Vec<Tile>) {
    let step_x = (tile.0 - overlap).max(0.1);
    let step_y = (tile.1 - overlap).max(0.1);
    let cols = (((content.0 - overlap) / step_x).ceil() as usize).max(1);
    let rows = (((content.1 - overlap) / step_y).ceil() as usize).max(1);
    let mut tiles = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            tiles.push(Tile {
                col,
                row,
                x: col as f64 * step_x,
                y: row as f64 * step_y,
            });
        }
    }
    (cols, rows, tiles)
}

/// `A1`, `B2`, … (row letter, 1-based column).
pub fn tile_coordinate(t: &Tile) -> String {
    let mut row = String::new();
    let mut r = t.row;
    loop {
        row.insert(0, (b'A' + (r % 26) as u8) as char);
        if r < 26 {
            break;
        }
        r = r / 26 - 1;
    }
    format!("{row}{}", t.col + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_and_scale() {
        let mut s = PrintSettings::default();
        assert_eq!(s.paper_cm(), (21.0, 29.7));
        s.orientation = Orientation::Landscape;
        assert_eq!(s.paper_cm(), (29.7, 21.0));
        s.orientation = Orientation::Portrait;
        // fit 60 cm circle on A4: (21-2)/60
        assert!((s.effective_scale(60.0, 60.0) - 19.0 / 60.0).abs() < 1e-12);
        s.scale_mode = ScaleMode::Actual;
        s.correction_factor = correction_factor(10.0, 9.8);
        assert!((s.effective_scale(60.0, 60.0) - 10.0 / 9.8).abs() < 1e-12);
        assert_eq!(correction_factor(10.0, 0.0), 1.0);
    }

    #[test]
    fn tiles() {
        // 60×60 content on 19×27.7 tiles with 1 cm overlap → 4 cols × 3 rows
        let (cols, rows, tiles) = tile_grid((60.0, 60.0), (19.0, 27.7), 1.0);
        assert_eq!((cols, rows), (4, 3));
        assert_eq!(
            tiles[5],
            Tile {
                col: 1,
                row: 1,
                x: 18.0,
                y: 26.7
            }
        );
        assert_eq!(tile_coordinate(&tiles[5]), "B2");
        assert_eq!(
            tile_coordinate(&Tile {
                col: 0,
                row: 26,
                x: 0.0,
                y: 0.0
            }),
            "AA1"
        );
    }
}
