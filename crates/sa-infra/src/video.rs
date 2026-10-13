//! Play-mode video export (spec 19) through a system FFmpeg: each frame is
//! rendered with tiny-skia and piped as raw RGBA; frames are held for the
//! configured interval at 30 fps. WebM (VP9) or MP4 (H.264) by extension.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use ffmpeg_sidecar::command::FfmpegCommand;

use sa_app::Document;
use sa_app::playback::{document_at_frame, total_thread_frames};
use sa_app::print::PrintElements;
use sa_render::Rgba;
use sa_render::raster::{self, Viewport};
use sa_render::scene::{SceneOptions, Style, build_scene};

pub const VIDEO_FPS: u32 = 30;
pub const VIDEO_DPI: f64 = 150.0;

/// Locates `ffmpeg`: PATH first, then common install dirs (GUI apps on macOS
/// don't inherit the shell PATH).
pub fn find_ffmpeg() -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.extend(
        [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "C:\\ffmpeg\\bin",
        ]
        .map(PathBuf::from),
    );
    dirs.into_iter().map(|d| d.join(exe)).find(|p| p.is_file())
}

/// Shared progress for the UI: frames done / total, cancel flag.
#[derive(Clone, Default)]
pub struct Progress {
    pub done: Arc<AtomicUsize>,
    pub total: Arc<AtomicUsize>,
    pub cancel: Arc<AtomicBool>,
}

/// Renders every frame (0..=total) and encodes `out`. Blocking — run on a
/// worker thread.
pub fn export_video(
    doc: &Document,
    out: &Path,
    interval_ms: f64,
    progress: &Progress,
) -> Result<(), String> {
    let ffmpeg = find_ffmpeg().ok_or("FFmpeg not found")?;
    let total = total_thread_frames(&doc.thread_layers);
    progress.total.store(total + 1, Ordering::Relaxed);
    let elements = PrintElements {
        pin_numbers: false,
        grid: false,
        ..PrintElements::default()
    };
    let opts = SceneOptions {
        style: Style::Editor,
        ..SceneOptions::export(elements)
    };
    let first = build_scene(doc, &opts);
    let mut vp = Viewport::for_scene_at_dpi(&first, VIDEO_DPI);
    // H.264/VP9 with yuv420p need even dimensions.
    vp.width += vp.width % 2;
    vp.height += vp.height % 2;
    let repeat = ((interval_ms / 1000.0) * VIDEO_FPS as f64).round().max(1.0) as usize;

    let mp4 = out
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"));
    let mut cmd = FfmpegCommand::new_with_path(ffmpeg);
    cmd.args(["-loglevel", "error"])
        .format("rawvideo")
        .pix_fmt("rgba")
        .size(vp.width, vp.height)
        .rate(VIDEO_FPS as f32)
        .input("-");
    if mp4 {
        cmd.codec_video("libx264").args([
            "-preset",
            "medium",
            "-crf",
            "18",
            "-movflags",
            "+faststart",
        ]);
    } else {
        cmd.codec_video("libvpx-vp9")
            .args(["-b:v", "0", "-crf", "30", "-row-mt", "1"]);
    }
    cmd.pix_fmt("yuv420p").overwrite().output(out);
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let mut stdin = child.take_stdin().ok_or("no ffmpeg stdin")?;
    let stderr = child.as_inner_mut().stderr.take();
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut e) = stderr {
            let _ = e.read_to_string(&mut s);
        }
        s
    });

    let mut result = Ok(());
    for frame in 0..=total {
        if progress.cancel.load(Ordering::Relaxed) {
            result = Err("cancelled".into());
            break;
        }
        let scene = build_scene(&document_at_frame(doc, frame), &opts);
        let Some(px) = raster::render(&scene, &vp, Rgba::rgb(0x11, 0x14, 0x19)) else {
            result = Err("render failed".into());
            break;
        };
        // Opaque background → premultiplied == straight alpha.
        for _ in 0..repeat {
            if let Err(e) = stdin.write_all(px.data()) {
                result = Err(e.to_string());
                break;
            }
        }
        if result.is_err() {
            break;
        }
        progress.done.store(frame + 1, Ordering::Relaxed);
    }
    drop(stdin);
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let err = err_thread.join().unwrap_or_default();
    result?;
    if !status.success() {
        return Err(if err.trim().is_empty() {
            format!("ffmpeg exited with {status}")
        } else {
            err
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_app::document::{PinPath, PinPathGeometry, PinStyle, SymmetryConfig, ThreadPath};
    use sa_domain::Point;

    #[test]
    fn encodes_webm_and_mp4_when_ffmpeg_available() {
        if find_ffmpeg().is_none() {
            return;
        }
        let mut doc = Document::empty();
        doc.board.dimensions.diameter = Some(10.0);
        let p = PinPath::new(
            PinPathGeometry::Circle {
                center: Point::ORIGIN,
                radius: 4.0,
            },
            2.0,
            &PinStyle {
                colour: "#fff".into(),
                diameter: 2.0,
                guide_visible: true,
            },
            SymmetryConfig::None,
        );
        let ids = p.pins.iter().take(4).map(|q| q.id.clone()).collect();
        doc.pin_layers[0].pin_paths.push(p);
        doc.thread_layers[0]
            .thread_paths
            .push(ThreadPath::new(ids, vec!["#f00".into()], 1.0, 6.0));
        for ext in ["webm", "mp4"] {
            let out =
                std::env::temp_dir().join(format!("sa-video-test-{}.{ext}", std::process::id()));
            let progress = Progress::default();
            export_video(&doc, &out, 100.0, &progress).unwrap();
            assert_eq!(progress.done.load(Ordering::Relaxed), 4);
            assert!(std::fs::metadata(&out).unwrap().len() > 100);
            let _ = std::fs::remove_file(out);
        }
    }
}
