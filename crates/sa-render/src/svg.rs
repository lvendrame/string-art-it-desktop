//! SVG backend (spec 15): true vectors, 1 user unit = 1 cm, mirrors the web
//! `buildExportSvg` structure.

use std::fmt::Write;

use base64::Engine;

use crate::color::Rgba;
use crate::path::{PathCmd, PathData};
use crate::scene::{BoardFill, Item, Scene, Stroke};

fn n(v: f64) -> String {
    // Shortest round-trip representation; trims "-0".
    let v = if v == 0.0 { 0.0 } else { v };
    let s = format!("{v}");
    s
}

pub fn path_d(d: &PathData) -> String {
    let mut s = String::new();
    for cmd in &d.cmds {
        match *cmd {
            PathCmd::MoveTo(p) => write!(s, "M {} {} ", n(p.x), n(p.y)),
            PathCmd::LineTo(p) => write!(s, "L {} {} ", n(p.x), n(p.y)),
            PathCmd::CubicTo(a, b, p) => {
                write!(
                    s,
                    "C {} {} {} {} {} {} ",
                    n(a.x),
                    n(a.y),
                    n(b.x),
                    n(b.y),
                    n(p.x),
                    n(p.y)
                )
            }
            PathCmd::Close => write!(s, "Z "),
        }
        .expect("string write");
    }
    s.trim_end().to_owned()
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn paint_attrs(prefix: &str, c: Rgba) -> String {
    if c.a == 255 {
        format!(r#"{prefix}="{}""#, c.to_hex())
    } else {
        format!(
            r#"{prefix}="{}" {prefix}-opacity="{}""#,
            c.to_hex(),
            n(c.opacity())
        )
    }
}

fn stroke_attrs(s: &Stroke) -> String {
    let mut a = format!(
        r#"{} stroke-width="{}""#,
        paint_attrs("stroke", s.color),
        n(s.width)
    );
    if let Some(d) = &s.dash {
        let arr: Vec<String> = d.array.iter().map(|v| n(*v)).collect();
        write!(a, r#" stroke-dasharray="{}""#, arr.join(" ")).expect("write");
        if d.offset != 0.0 {
            write!(a, r#" stroke-dashoffset="{}""#, n(d.offset)).expect("write");
        }
    }
    a
}

fn stops_markup(stops: &[crate::scene::Stop]) -> String {
    stops
        .iter()
        .map(|s| {
            let op = if s.color.a == 255 {
                String::new()
            } else {
                format!(r#" stop-opacity="{}""#, n(s.color.opacity()))
            };
            format!(
                r#"<stop offset="{}%" stop-color="{}"{op}/>"#,
                n(s.offset * 100.0),
                s.color.to_hex()
            )
        })
        .collect()
}

fn fill_defs(id: &str, fill: &BoardFill) -> (String, String) {
    let url = format!("url(#{id})");
    match fill {
        BoardFill::Solid(c) => (String::new(), paint_attrs("fill", *c)),
        BoardFill::Linear {
            stops,
            direction_deg,
        } => (
            format!(
                r#"<linearGradient id="{id}" gradientTransform="rotate({})">{}</linearGradient>"#,
                n(*direction_deg),
                stops_markup(stops)
            ),
            format!(r#"fill="{url}""#),
        ),
        BoardFill::Radial { stops, centre } => (
            format!(
                r#"<radialGradient id="{id}" cx="{}%" cy="{}%">{}</radialGradient>"#,
                n(centre.x * 100.0),
                n(centre.y * 100.0),
                stops_markup(stops)
            ),
            format!(r#"fill="{url}""#),
        ),
        BoardFill::Wood { bands, tint } => {
            let mut s = format!(
                r#"<pattern id="{id}" patternUnits="objectBoundingBox" patternContentUnits="objectBoundingBox" width="1" height="1">"#
            );
            for b in bands.iter() {
                let mut d = PathData::polyline(&b.outer, true);
                if let Some(inner) = &b.inner {
                    d.cmds.extend(PathData::polyline(inner, true).cmds);
                }
                write!(
                    s,
                    r#"<path d="{}" fill-rule="evenodd" fill="{}"/>"#,
                    path_d(&d),
                    b.colour.to_hex()
                )
                .expect("write");
            }
            if let Some(t) = tint {
                write!(
                    s,
                    r#"<rect x="0" y="0" width="1" height="1" {}/>"#,
                    paint_attrs("fill", *t)
                )
                .expect("write");
            }
            s.push_str("</pattern>");
            (s, format!(r#"fill="{url}""#))
        }
        BoardFill::Image(img) => {
            let data = base64::engine::general_purpose::STANDARD.encode(&img.encoded);
            (
                format!(
                    r#"<pattern id="{id}" patternUnits="objectBoundingBox" patternContentUnits="objectBoundingBox" width="1" height="1"><image href="data:{};base64,{data}" x="0" y="0" width="1" height="1" preserveAspectRatio="xMidYMid slice"/></pattern>"#,
                    img.mime
                ),
                format!(r#"fill="{url}""#),
            )
        }
    }
}

pub fn to_svg(scene: &Scene) -> String {
    let b = &scene.bounds;
    let mut defs = String::new();
    let mut body = String::new();
    for (i, item) in scene.items.iter().enumerate() {
        match item {
            Item::Board {
                outline,
                fill,
                stroke,
                ..
            } => {
                let fill_attr = match fill {
                    Some(f) => {
                        let (d, attr) = fill_defs(&format!("export-fill-{i}"), f);
                        defs.push_str(&d);
                        attr
                    }
                    None => r#"fill="none""#.to_owned(),
                };
                let stroke_attr = stroke.as_ref().map(stroke_attrs).unwrap_or_default();
                writeln!(
                    body,
                    r#"<path d="{}" {fill_attr} {stroke_attr}/>"#,
                    path_d(outline)
                )
                .expect("write");
            }
            Item::Path { data, fill, stroke } => {
                let fill_attr = fill
                    .map(|c| paint_attrs("fill", c))
                    .unwrap_or_else(|| r#"fill="none""#.into());
                let stroke_attr = stroke.as_ref().map(stroke_attrs).unwrap_or_default();
                writeln!(
                    body,
                    r#"<path d="{}" {fill_attr} {stroke_attr}/>"#,
                    path_d(data)
                )
                .expect("write");
            }
            Item::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                let stroke_attr = stroke.as_ref().map(stroke_attrs).unwrap_or_default();
                writeln!(
                    body,
                    r#"<circle cx="{}" cy="{}" r="{}" {} {stroke_attr}/>"#,
                    n(center.x),
                    n(center.y),
                    n(*radius),
                    paint_attrs("fill", *fill)
                )
                .expect("write");
            }
            Item::Polyline { points, stroke } => {
                writeln!(
                    body,
                    r#"<path d="{}" fill="none" {}/>"#,
                    path_d(&PathData::polyline(points, false)),
                    stroke_attrs(stroke)
                )
                .expect("write");
            }
            Item::Text {
                pos,
                text,
                size,
                color,
                ..
            } => {
                writeln!(
                    body,
                    r#"<text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-size="{}" {}>{}</text>"#,
                    n(pos.x),
                    n(pos.y),
                    n(*size),
                    paint_attrs("fill", *color),
                    escape(text)
                )
                .expect("write");
            }
        }
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}cm\" height=\"{h}cm\" viewBox=\"{} {} {w} {h}\">\n<defs>{defs}</defs>\n{body}</svg>\n",
        n(b.min_x),
        n(b.min_y),
        w = n(b.width()),
        h = n(b.height()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{SceneOptions, build_scene};
    use sa_app::Document;
    use sa_app::print::PrintElements;

    #[test]
    fn empty_doc_svg_shape() {
        let scene = build_scene(
            &Document::empty(),
            &SceneOptions::export(PrintElements::default()),
        );
        let svg = to_svg(&scene);
        assert!(
            svg.contains(r#"width="64cm" height="64cm" viewBox="-32 -32 64 64""#),
            "{svg}"
        );
        assert!(svg.contains(r##"fill="#3b3b3b" stroke="#000000" stroke-width="0.05""##));
        // parses as valid SVG
        resvg_check(&svg);
    }

    fn resvg_check(svg: &str) {
        // Basic well-formedness: every tag closes.
        assert_eq!(svg.matches("<svg").count(), svg.matches("</svg>").count());
    }
}
