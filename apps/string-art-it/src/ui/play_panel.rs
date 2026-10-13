//! Play mode (spec 19): transport, frame/interval fields, video export with
//! progress, and the read-only playback canvas.

use std::sync::atomic::Ordering;

use egui::Ui;
use sa_app::editor::Store;
use sa_app::playback::{Transport, total_thread_frames};
use sa_infra::video::{Progress, export_video, find_ffmpeg};

use crate::fonts;
use crate::icons::Icon;
use crate::ui::radial_menu::PlayCommand;
use crate::widgets::{self, Kit};

#[derive(Default)]
pub struct PlayState {
    pub transport: Transport,
    export: Option<(Progress, std::thread::JoinHandle<Result<(), String>>)>,
    pub message: Option<String>,
}

impl PlayState {
    pub fn exporting(&self) -> bool {
        self.export.is_some()
    }

    pub fn command(&mut self, cmd: PlayCommand, store: &Store, now: f64) {
        if self.exporting() {
            return;
        }
        let total = total_thread_frames(&store.doc.thread_layers);
        let t = &mut self.transport;
        match cmd {
            PlayCommand::First => t.go_to(0, total),
            PlayCommand::Previous => t.go_to(t.frame.saturating_sub(1), total),
            PlayCommand::Toggle => t.toggle(total, now),
            PlayCommand::Next => t.go_to(t.frame + 1, total),
            PlayCommand::Last => t.go_to(total, total),
        }
    }

    /// Per-frame: advance playback, poll export.
    pub fn tick(&mut self, ctx: &egui::Context, store: &Store, kit: &Kit) {
        let total = total_thread_frames(&store.doc.thread_layers);
        let now = ctx.input(|i| i.time);
        if let Some(wait) = self.transport.tick(total, now) {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait.max(0.001)));
        }
        if let Some((progress, handle)) = &self.export {
            if handle.is_finished() {
                let (_, handle) = self.export.take().expect("export");
                self.message = Some(match handle.join() {
                    Ok(Ok(())) => kit.tr("desktop:video.saved"),
                    Ok(Err(e)) if e == "cancelled" => String::new(),
                    Ok(Err(e)) => e,
                    Err(_) => "export crashed".into(),
                });
            } else {
                self.transport.frame = progress
                    .done
                    .load(Ordering::Relaxed)
                    .saturating_sub(1)
                    .min(total);
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
    }

    pub fn start_export(&mut self, store: &Store, kit: &Kit) {
        if self.exporting() || total_thread_frames(&store.doc.thread_layers) == 0 {
            return;
        }
        if find_ffmpeg().is_none() {
            self.message = Some(kit.tr("desktop:video.ffmpegMissing"));
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("WebM", &["webm"])
            .add_filter("MP4", &["mp4"])
            .set_file_name("string-art-animation.webm")
            .save_file()
        else {
            return;
        };
        self.transport.playing = false;
        let doc = store.doc.clone();
        let interval = self.transport.interval_ms;
        let progress = Progress::default();
        let p2 = progress.clone();
        let handle = std::thread::spawn(move || export_video(&doc, &path, interval, &p2));
        self.message = None;
        self.export = Some((progress, handle));
    }

    pub fn panel(&mut self, ui: &mut Ui, kit: &mut Kit, store: &Store) {
        ui.spacing_mut().item_spacing.y = 8.0;
        widgets::section_label(ui, kit, &kit.tr("toolbars:playToolbar.sectionTitle"));
        let total = total_thread_frames(&store.doc.thread_layers);
        let now = ui.input(|i| i.time);
        let enabled = !self.exporting() && total > 0;
        let mut cmd = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let at_end = self.transport.frame >= total;
            let play_tip = if self.transport.playing {
                "toolbars:playToolbar.pause"
            } else if at_end {
                "toolbars:playToolbar.playRestart"
            } else {
                "toolbars:playToolbar.play"
            };
            for (icon, tip, c) in [
                (
                    Icon::SkipBack,
                    "toolbars:playToolbar.firstFrame",
                    PlayCommand::First,
                ),
                (
                    Icon::ChevronLeft,
                    "toolbars:playToolbar.previousFrame",
                    PlayCommand::Previous,
                ),
                (
                    if self.transport.playing {
                        Icon::Pause
                    } else {
                        Icon::Play
                    },
                    play_tip,
                    PlayCommand::Toggle,
                ),
                (
                    Icon::ChevronRight,
                    "toolbars:playToolbar.nextFrame",
                    PlayCommand::Next,
                ),
                (
                    Icon::SkipForward,
                    "toolbars:playToolbar.lastFrame",
                    PlayCommand::Last,
                ),
            ] {
                if widgets::icon_button(ui, kit, icon, &kit.tr(tip), enabled).clicked() {
                    cmd = Some(c);
                }
            }
        });
        if let Some(c) = cmd {
            self.command(c, store, now);
        }
        widgets::property_row(ui, kit, &kit.tr("toolbars:playToolbar.frame"), |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                if let Some(v) = widgets::number(
                    ui,
                    kit,
                    self.transport.frame as f64,
                    0.0..=total as f64,
                    0.2,
                    0,
                ) {
                    self.transport.go_to(v.round() as usize, total);
                }
            });
        });
        widgets::property_row(ui, kit, &kit.tr("toolbars:playToolbar.total"), |ui| {
            ui.label(egui::RichText::new(total.to_string()).font(fonts::mono(12.0)));
        });
        widgets::property_row(
            ui,
            kit,
            &kit.tr("toolbars:playToolbar.timeBetweenFrames"),
            |ui| {
                ui.add_enabled_ui(!self.exporting(), |ui| {
                    if let Some(v) = widgets::number(
                        ui,
                        kit,
                        self.transport.interval_ms,
                        10.0..=10_000.0,
                        1.0,
                        0,
                    ) {
                        self.transport.interval_ms = v;
                    }
                });
            },
        );
        ui.add_space(6.0);
        if let Some((p, _)) = &self.export {
            let (done, all) = (
                p.done.load(Ordering::Relaxed),
                p.total.load(Ordering::Relaxed).max(1),
            );
            ui.label(kit.trf(
                "toolbars:playToolbar.exporting",
                &[("progress", &done), ("total", &all)],
            ));
            ui.add(egui::ProgressBar::new(done as f32 / all as f32));
            if ui.button(kit.tr("desktop:video.cancel")).clicked() {
                p.cancel.store(true, Ordering::Relaxed);
            }
        } else {
            let label = format!("{} [Shift+E]", kit.tr("toolbars:playToolbar.exportToVideo"));
            if widgets::button(ui, kit, Some(Icon::Video), Some(&label), false, total > 0).clicked()
            {
                self.start_export(store, kit);
            }
        }
        if let Some(m) = self.message.as_ref().filter(|m| !m.is_empty()) {
            ui.label(
                egui::RichText::new(m)
                    .size(11.5)
                    .color(kit.t.text_secondary),
            );
        }
    }

    /// Play-mode keys (spec 34): F/P/Space/N/L, Shift+E export.
    pub fn keys(&mut self, ctx: &egui::Context, store: &Store, kit: &Kit) {
        if ctx.egui_wants_keyboard_input() || self.exporting() {
            return;
        }
        let now = ctx.input(|i| i.time);
        let events: Vec<(egui::Key, egui::Modifiers)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        repeat: false,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                })
                .collect()
        });
        for (k, m) in events {
            if m.command || m.alt {
                continue;
            }
            match (k, m.shift) {
                (egui::Key::E, true) => self.start_export(store, kit),
                (egui::Key::F, false) => self.command(PlayCommand::First, store, now),
                (egui::Key::P, false) => self.command(PlayCommand::Previous, store, now),
                (egui::Key::Space, false) => self.command(PlayCommand::Toggle, store, now),
                (egui::Key::N, false) => self.command(PlayCommand::Next, store, now),
                (egui::Key::L, false) => self.command(PlayCommand::Last, store, now),
                _ => {}
            }
        }
    }
}
