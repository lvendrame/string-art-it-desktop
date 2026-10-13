//! CSS colour strings (user data from the document) → RGBA.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const BLACK: Rgba = Rgba::rgb(0, 0, 0);
    pub const WHITE: Rgba = Rgba::rgb(255, 255, 255);
    pub const TRANSPARENT: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn with_opacity(self, opacity: f64) -> Self {
        Self {
            a: (self.a as f64 * opacity.clamp(0.0, 1.0)).round() as u8,
            ..self
        }
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn opacity(self) -> f64 {
        self.a as f64 / 255.0
    }

    /// Parses `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()/rgba()` and the CSS
    /// named colours used in practice. Unknown values fall back to black (browser
    /// behaviour for an invalid fill is "none"; black keeps pins visible).
    pub fn parse(s: &str) -> Rgba {
        Self::try_parse(s).unwrap_or(Rgba::BLACK)
    }

    pub fn try_parse(s: &str) -> Option<Rgba> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix('#') {
            let nib = |c: u8| (c as char).to_digit(16).map(|v| v as u8);
            let b = hex.as_bytes();
            let full = |i: usize| Some(nib(b[i])? * 16 + nib(b[i + 1])?);
            return match b.len() {
                3 | 4 => {
                    let ch = |i: usize| nib(b[i]).map(|v| v * 17);
                    Some(Rgba {
                        r: ch(0)?,
                        g: ch(1)?,
                        b: ch(2)?,
                        a: if b.len() == 4 { ch(3)? } else { 255 },
                    })
                }
                6 | 8 => Some(Rgba {
                    r: full(0)?,
                    g: full(2)?,
                    b: full(4)?,
                    a: if b.len() == 8 { full(6)? } else { 255 },
                }),
                _ => None,
            };
        }
        let lower = s.to_ascii_lowercase();
        if let Some(args) = lower
            .strip_prefix("rgba(")
            .or_else(|| lower.strip_prefix("rgb("))
            .and_then(|r| r.strip_suffix(')'))
        {
            let parts: Vec<&str> = args
                .split([',', ' ', '/'])
                .filter(|p| !p.is_empty())
                .collect();
            let comp = |p: &str| p.trim().parse::<f64>().ok();
            let r = comp(parts.first()?)?;
            let g = comp(parts.get(1)?)?;
            let b = comp(parts.get(2)?)?;
            let a = parts.get(3).and_then(|p| comp(p)).unwrap_or(1.0);
            let c = |v: f64| v.round().clamp(0.0, 255.0) as u8;
            return Some(Rgba {
                r: c(r),
                g: c(g),
                b: c(b),
                a: (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            });
        }
        Some(match lower.as_str() {
            "black" => Rgba::rgb(0, 0, 0),
            "white" => Rgba::rgb(255, 255, 255),
            "red" => Rgba::rgb(255, 0, 0),
            "green" => Rgba::rgb(0, 128, 0),
            "blue" => Rgba::rgb(0, 0, 255),
            "yellow" => Rgba::rgb(255, 255, 0),
            "orange" => Rgba::rgb(255, 165, 0),
            "purple" => Rgba::rgb(128, 0, 128),
            "gray" | "grey" => Rgba::rgb(128, 128, 128),
            "transparent" | "none" => Rgba::TRANSPARENT,
            _ => return None,
        })
    }
}

/// Interpolates two colours channel-wise with JS `Math.round` semantics, matching
/// the web wood-grain `mixHex`.
pub fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let l = |x: u8, y: u8| {
        let v = x as f64 + t * (y as f64 - x as f64);
        (v + 0.5).floor().clamp(0.0, 255.0) as u8
    };
    Rgba::rgb(l(a.r, b.r), l(a.g, b.g), l(a.b, b.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        assert_eq!(Rgba::parse("#fff"), Rgba::WHITE);
        assert_eq!(Rgba::parse("#6d5ef7"), Rgba::rgb(0x6d, 0x5e, 0xf7));
        assert_eq!(Rgba::parse("#00000080").a, 0x80);
        assert_eq!(Rgba::parse("red"), Rgba::rgb(255, 0, 0));
        assert_eq!(Rgba::parse("rgb(1, 2, 3)"), Rgba::rgb(1, 2, 3));
        assert_eq!(Rgba::parse("rgba(1,2,3,0.5)").a, 128);
        assert_eq!(Rgba::parse("garbage"), Rgba::BLACK);
        assert_eq!(Rgba::parse("#6d5ef7").to_hex(), "#6d5ef7");
    }

    #[test]
    fn mix_matches_web() {
        // mixHex("#000000", "#ffffff", 0.5) = #808080 (127.5 rounds up)
        assert_eq!(mix(Rgba::BLACK, Rgba::WHITE, 0.5), Rgba::rgb(128, 128, 128));
    }
}
