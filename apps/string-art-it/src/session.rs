//! File session (spec 16, desktop): current path, dirty tracking, window
//! title, autosave + restore offer, unsaved-changes confirmation.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use sa_app::document::{ProjectError, ProjectFile};
use sa_app::editor::Store;
use sa_infra::persistence::{Autosave, FileError, read_project, write_project};

use crate::widgets::Kit;

const AUTOSAVE_DEBOUNCE: Duration = Duration::from_secs(2);

/// An action waiting on the unsaved-changes prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guarded {
    New,
    Open,
    Close,
}

pub enum Confirm {
    Save,
    DontSave,
    Cancel,
}

pub struct Session {
    pub path: Option<PathBuf>,
    saved_revision: u64,
    autosave: Option<Autosave>,
    autosaved_revision: u64,
    seen_revision: u64,
    last_change: Instant,
    pub pending: Option<Guarded>,
    pub restore_offer: Option<ProjectFile>,
    pub error: Option<String>,
    pub allow_close: bool,
    title: String,
}

impl Session {
    pub fn new(persist: bool) -> Self {
        let autosave = persist.then(Autosave::default);
        let restore_offer = autosave.as_ref().and_then(Autosave::load);
        Self {
            path: None,
            saved_revision: 0,
            autosave,
            autosaved_revision: 0,
            seen_revision: 0,
            last_change: Instant::now(),
            pending: None,
            restore_offer,
            error: None,
            allow_close: false,
            title: String::new(),
        }
    }

    pub fn is_dirty(&self, store: &Store) -> bool {
        store.revision() != self.saved_revision
    }

    pub fn mark_clean(&mut self, store: &Store) {
        self.saved_revision = store.revision();
        self.autosaved_revision = store.revision();
        if let Some(a) = &self.autosave {
            a.clear();
        }
    }

    pub fn reset(&mut self, store: &Store) {
        self.path = None;
        self.mark_clean(store);
    }

    /// Per-frame: title + debounced autosave.
    pub fn tick(&mut self, ctx: &egui::Context, store: &Store) {
        let rev = store.revision();
        if rev != self.seen_revision {
            self.seen_revision = rev;
            self.last_change = Instant::now();
        }
        if self.is_dirty(store) && rev != self.autosaved_revision {
            let elapsed = self.last_change.elapsed();
            if elapsed >= AUTOSAVE_DEBOUNCE {
                if let Some(a) = &self.autosave {
                    a.save(&store.to_project_file());
                }
                self.autosaved_revision = rev;
            } else {
                ctx.request_repaint_after(AUTOSAVE_DEBOUNCE - elapsed);
            }
        }
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into());
        let title = format!(
            "{}{name} — StringArtIt",
            if self.is_dirty(store) { "• " } else { "" }
        );
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    pub fn error_text(kit: &Kit, e: &FileError) -> String {
        match e {
            FileError::Project(ProjectError::Invalid(_)) => kit.tr("errors:openFile.invalid"),
            FileError::Project(ProjectError::IncompatibleVersion(v)) => kit.trf(
                "errors:openFile.incompatibleVersion",
                &[
                    ("found", v),
                    ("max", &sa_app::document::project::CURRENT_PROJECT_VERSION),
                ],
            ),
            FileError::Project(ProjectError::NoMigrationPath(v)) => {
                kit.trf("errors:openFile.noMigrationPath", &[("version", v)])
            }
            FileError::Io(_) => kit.tr("errors:openFile.unknown"),
        }
    }

    fn dialog(kit: &Kit) -> rfd::FileDialog {
        rfd::FileDialog::new().add_filter(kit.tr("desktop:file.projectFiles"), &["json"])
    }

    /// Native open dialog → parsed project.
    pub fn pick_and_read(&mut self, kit: &Kit) -> Option<ProjectFile> {
        let path = Self::dialog(kit).pick_file()?;
        match read_project(&path) {
            Ok(f) => {
                self.path = Some(path);
                Some(f)
            }
            Err(e) => {
                self.error = Some(Self::error_text(kit, &e));
                None
            }
        }
    }

    /// Save (or Save As when untitled / `force_dialog`). Returns success.
    pub fn save(&mut self, kit: &Kit, store: &Store, force_dialog: bool) -> bool {
        let path = match (&self.path, force_dialog) {
            (Some(p), false) => p.clone(),
            _ => match Self::dialog(kit)
                .set_file_name("string-art-project.json")
                .save_file()
            {
                Some(p) => p,
                None => return false,
            },
        };
        match write_project(&path, &store.to_project_file()) {
            Ok(()) => {
                self.path = Some(path);
                self.mark_clean(store);
                true
            }
            Err(e) => {
                self.error = Some(e.to_string());
                false
            }
        }
    }

    /// Modal unsaved-changes prompt; returns the user's answer when given.
    pub fn confirm_modal(&self, ctx: &egui::Context, kit: &Kit) -> Option<Confirm> {
        self.pending?;
        let mut out = None;
        egui::Modal::new(egui::Id::new("unsaved-modal")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.heading(kit.tr("desktop:unsaved.title"));
            ui.label(kit.tr("desktop:unsaved.body"));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(kit.tr("desktop:unsaved.save")).clicked() {
                    out = Some(Confirm::Save);
                }
                if ui.button(kit.tr("desktop:unsaved.dontSave")).clicked() {
                    out = Some(Confirm::DontSave);
                }
                if ui.button(kit.tr("desktop:unsaved.cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    out = Some(Confirm::Cancel);
                }
            });
        });
        out
    }

    /// Autosave restore banner (web AutosaveDialog). Returns Some(true) = restore.
    pub fn restore_modal(&self, ctx: &egui::Context, kit: &Kit) -> Option<bool> {
        self.restore_offer.as_ref()?;
        let mut out = None;
        egui::Modal::new(egui::Id::new("restore-modal")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(kit.tr("common:autosave.found"));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(kit.tr("common:autosave.restore")).clicked() {
                    out = Some(true);
                }
                if ui.button(kit.tr("common:autosave.discard")).clicked() {
                    out = Some(false);
                }
            });
        });
        out
    }

    pub fn error_modal(&mut self, ctx: &egui::Context) {
        let Some(msg) = self.error.clone() else {
            return;
        };
        let r = egui::Modal::new(egui::Id::new("error-modal")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(msg);
            ui.add_space(8.0);
            ui.button("OK").clicked()
        });
        if r.inner || r.should_close() {
            self.error = None;
        }
    }

    pub fn discard_autosave(&mut self) {
        self.restore_offer = None;
        if let Some(a) = &self.autosave {
            a.clear();
        }
    }
}
