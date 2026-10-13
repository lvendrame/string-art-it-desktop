//! Application root: board setup → editor shell.

use sa_app::editor::{EditorMode, Store};
use sa_infra::i18n::{I18n, Language};

use crate::canvas::CanvasView;
use crate::fonts;
use crate::icons::IconCache;
use crate::session::{Confirm, Guarded, Session};
use crate::theme::{self, Tokens};
use crate::tools::pin::PinToolState;
use crate::tools::select::SelectToolState;
use crate::tools::thread::ThreadToolState;
use crate::ui::board_setup::{SetupAction, board_setup};
use crate::ui::layers_panel::LayersPanelState;
use crate::ui::pin_panel::pin_panel;
use crate::ui::radial_menu::RadialMenu;
use crate::ui::select_panel::select_panel;
use crate::ui::shell::{self, ShellAction};
use crate::ui::thread_panel::thread_panel;
use crate::widgets::Kit;

/// Full-bleed overlays over the editor body (web OverlayPanel).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Print,
    Stats,
    Help,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Setup,
    Editor,
}

pub struct StringArtApp {
    tokens: Tokens,
    icons: IconCache,
    i18n: I18n,
    store: Store,
    screen: Screen,
    canvas: CanvasView,
    pin_tool: PinToolState,
    select_tool: SelectToolState,
    thread_tool: ThreadToolState,
    layers_panel: LayersPanelState,
    radial: RadialMenu,
    session: Session,
    prefs: Option<sa_infra::persistence::Prefs>,
    export_dpi: f64,
    overlay: Overlay,
    print_panel: crate::ui::print_panel::PrintPanel,
    play: crate::ui::play_panel::PlayState,
    last_mode: EditorMode,
    generator: crate::ui::generator_panel::GeneratorPanel,
    help: crate::ui::help_panel::HelpPanel,
    /// Language the installed font fallbacks were built for.
    fonts_lang: Option<Language>,
    agent: crate::agent::Agent,
}

impl StringArtApp {
    /// The real app: persistence (autosave, prefs) enabled.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::with_context(&cc.egui_ctx);
        let prefs = sa_infra::persistence::Prefs::load();
        let lang = Language::detect(
            sys_locale::get_locale().as_deref(),
            prefs.language.as_deref(),
        );
        app.i18n = I18n::new(lang);
        app.session = Session::new(true);
        if let Some(f) = prefs.calibration_factor {
            app.store.editor.print.correction_factor = f;
        }
        let agent_on = prefs.agent_server;
        app.prefs = Some(prefs);
        if agent_on {
            app.set_agent(&cc.egui_ctx, true);
        }
        app
    }

    /// Without persistence (tests, automation).
    pub fn with_context(ctx: &egui::Context) -> Self {
        let tokens = Tokens::dark();
        theme::apply(ctx, &tokens);
        let lang = Language::detect(sys_locale::get_locale().as_deref(), None);
        fonts::install(ctx, lang);
        Self {
            tokens,
            icons: IconCache::default(),
            i18n: I18n::new(lang),
            store: Store::default(),
            screen: Screen::Setup,
            canvas: CanvasView::default(),
            pin_tool: PinToolState::default(),
            select_tool: SelectToolState::default(),
            thread_tool: ThreadToolState,
            layers_panel: LayersPanelState::default(),
            radial: RadialMenu::default(),
            session: Session::new(false),
            prefs: None,
            export_dpi: sa_infra::export::DEFAULT_DPI,
            overlay: Overlay::None,
            fonts_lang: Some(lang),
            agent: Default::default(),
            print_panel: Default::default(),
            play: Default::default(),
            last_mode: EditorMode::Pin,
            generator: Default::default(),
            help: Default::default(),
        }
    }

    /// Test/automation hooks.
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    pub fn enter_editor(&mut self) {
        self.store.set_mode(EditorMode::Pin);
        self.canvas.request_fit();
        self.screen = Screen::Editor;
    }

    pub fn set_play_frame(&mut self, frame: usize) {
        self.last_mode = EditorMode::Play;
        self.play.transport.frame = frame;
    }

    pub fn open_overlay_for_test(&mut self, name: &str) {
        self.overlay = match name {
            "stats" => Overlay::Stats,
            "help" => {
                self.help.open(self.store.editor.mode);
                Overlay::Help
            }
            other => match other.strip_prefix("help:") {
                Some(tab) => {
                    self.help.set_tab(tab);
                    Overlay::Help
                }
                None => Overlay::None,
            },
        };
    }

    /// Starts the agent server on `port` (0 = any free port); returns its URL.
    pub fn start_agent_for_test(&mut self, ctx: &egui::Context, port: u16) -> Option<String> {
        self.agent.set_enabled(ctx, true, port);
        self.agent.status().url
    }

    pub fn open_print_preview(&mut self) {
        self.overlay = Overlay::Print;
    }

    /// Current viewport and canvas top-left in screen points (tests).
    pub fn canvas_origin(&self) -> (sa_domain::transforms::Viewport, egui::Pos2) {
        (self.store.editor.viewport, self.canvas.origin())
    }

    pub fn set_language_code(&mut self, code: &str) {
        if let Some(l) = Language::from_code(code) {
            self.set_language(l);
        }
    }

    fn set_language(&mut self, lang: Language) {
        self.i18n = I18n::new(lang);
        if let Some(p) = &mut self.prefs {
            p.language = Some(lang.code().to_owned());
            p.save();
        }
    }

    fn set_agent(&mut self, ctx: &egui::Context, on: bool) {
        let port = self
            .prefs
            .as_ref()
            .and_then(|p| p.agent_port)
            .unwrap_or(sa_infra::mcp::DEFAULT_PORT);
        self.agent.set_enabled(ctx, on, port);
        if let Some(p) = &mut self.prefs {
            p.agent_server = on && self.agent.error.is_none();
            p.save();
        }
    }

    /// Runs an action guarded by the unsaved-changes prompt.
    fn guarded(&mut self, ctx: &egui::Context, g: Guarded) {
        if self.session.is_dirty(&self.store)
            && !(g == Guarded::New && self.screen == Screen::Setup)
        {
            self.session.pending = Some(g);
        } else {
            self.perform(ctx, g);
        }
    }

    fn perform(&mut self, ctx: &egui::Context, g: Guarded) {
        match g {
            Guarded::New => {
                self.store = Store::default();
                self.session.reset(&self.store);
                self.screen = Screen::Setup;
            }
            Guarded::Open => {
                let kit = Kit {
                    t: self.tokens,
                    icons: &mut self.icons,
                    i18n: self.i18n,
                };
                if let Some(file) = self.session.pick_and_read(&kit) {
                    self.store.load_project(file);
                    self.session.mark_clean(&self.store);
                    self.enter_editor();
                }
            }
            Guarded::Close => {
                self.session.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn handle_shell_action(&mut self, ctx: &egui::Context, action: ShellAction) {
        match action {
            ShellAction::New => self.guarded(ctx, Guarded::New),
            ShellAction::Open => self.guarded(ctx, Guarded::Open),
            ShellAction::Save => {
                let kit = Kit {
                    t: self.tokens,
                    icons: &mut self.icons,
                    i18n: self.i18n,
                };
                self.session.save(&kit, &self.store, false);
            }
            ShellAction::Language(l) => self.set_language(l),
            ShellAction::Agent(on) => self.set_agent(ctx, on),
            // Wired in later milestones (D10–D15).
            ShellAction::Export(format) => self.export(format),
            ShellAction::Print => self.overlay = Overlay::Print,
            ShellAction::Stats => self.overlay = Overlay::Stats,
            ShellAction::Help => {
                self.help.open(self.store.editor.mode);
                self.overlay = Overlay::Help;
            }
        }
    }
}

impl StringArtApp {
    /// Spec 15: export with the print-element toggles, via a native save dialog.
    fn export(&mut self, format: sa_infra::export::ExportFormat) {
        use sa_infra::export::export;
        let stem = self
            .session
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "string-art".into());
        let Some(path) = rfd::FileDialog::new()
            .add_filter(format.extension().to_uppercase(), &[format.extension()])
            .set_file_name(format!("{stem}.{}", format.extension()))
            .save_file()
        else {
            return;
        };
        let result = export(
            &self.store.doc,
            self.store.editor.print.elements,
            format,
            self.export_dpi,
        )
        .map_err(|e| e.to_string())
        .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        if let Err(e) = result {
            let key = if format == sa_infra::export::ExportFormat::Pdf {
                "errors:export.pdfFailed"
            } else {
                "errors:export.rasterFailed"
            };
            self.session.error = Some(format!("{} ({e})", self.i18n.t(key)));
        }
    }

    fn session_frame(&mut self, ctx: &egui::Context) {
        self.session.tick(ctx, &self.store);
        // Window close with unsaved changes → prompt.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.session.allow_close
            && self.session.is_dirty(&self.store)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.session.pending = Some(Guarded::Close);
        }
        let kit = Kit {
            t: self.tokens,
            icons: &mut self.icons,
            i18n: self.i18n,
        };
        if let Some(answer) = self.session.confirm_modal(ctx, &kit) {
            let g = self.session.pending.take().expect("pending action");
            match answer {
                Confirm::Save => {
                    if self.session.save(&kit, &self.store, false) {
                        self.perform(ctx, g);
                    }
                }
                Confirm::DontSave => self.perform(ctx, g),
                Confirm::Cancel => {}
            }
        }
        let kit = Kit {
            t: self.tokens,
            icons: &mut self.icons,
            i18n: self.i18n,
        };
        match self.session.restore_modal(ctx, &kit) {
            Some(true) => {
                if let Some(f) = self.session.restore_offer.take() {
                    self.store.load_project(f);
                    // Restored work stays dirty until saved somewhere.
                    self.session.path = None;
                    self.enter_editor();
                }
            }
            Some(false) => self.session.discard_autosave(),
            None => {}
        }
        self.session.error_modal(ctx);
    }
}

impl eframe::App for StringArtApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.agent.pump(&mut self.store) {
            ui.ctx().request_repaint();
        }
        if self.fonts_lang != Some(self.i18n.lang) {
            fonts::install(ui.ctx(), self.i18n.lang);
            self.fonts_lang = Some(self.i18n.lang);
        }
        let t = self.tokens;
        // Any pointer release ends a coalescing edit (drag value, colour picker).
        if ui.input(|i| i.pointer.any_released()) {
            self.store.end_coalescing();
        }
        let mut kit = Kit {
            t,
            icons: &mut self.icons,
            i18n: self.i18n,
        };

        match self.screen {
            Screen::Setup => {
                let mut picked_lang = None;
                let action = egui::CentralPanel::no_frame()
                    .frame(egui::Frame::new().fill(t.bg_app))
                    .show(ui, |ui| {
                        board_setup(ui, &mut kit, &mut self.store, |ui, kit| {
                            picked_lang = shell::language_switcher(ui, kit);
                        })
                    })
                    .inner;
                if let Some(l) = picked_lang {
                    self.set_language(l);
                }
                match action {
                    SetupAction::Continue => {
                        self.store.set_mode(EditorMode::Pin);
                        self.canvas.request_fit();
                        self.screen = Screen::Editor;
                    }
                    SetupAction::CycleLanguage => {
                        let i = Language::ALL
                            .iter()
                            .position(|l| *l == self.i18n.lang)
                            .unwrap_or(0);
                        self.set_language(Language::ALL[(i + 1) % Language::ALL.len()]);
                    }
                    SetupAction::None => {}
                }
            }
            Screen::Editor => {
                let agent_status = self.agent.status();
                let action = egui::Panel::top("top-bar")
                    .exact_size(theme::TOP_BAR_HEIGHT)
                    .frame(
                        egui::Frame::new()
                            .fill(t.bg_panel)
                            .inner_margin(egui::Margin::symmetric(16, 0))
                            .stroke(egui::Stroke::new(1.0, t.border)),
                    )
                    .show(ui, |ui| {
                        shell::top_bar(
                            ui,
                            &mut kit,
                            &mut self.store,
                            &mut self.export_dpi,
                            &agent_status,
                        )
                    })
                    .inner;

                if self.overlay == Overlay::Print {
                    use crate::ui::print_panel::PrintPanelAction;
                    match self.print_panel.show(ui, &mut kit, &mut self.store) {
                        Some(PrintPanelAction::Close) => self.overlay = Overlay::None,
                        Some(PrintPanelAction::Calibrated(f)) => {
                            if let Some(p) = &mut self.prefs {
                                p.calibration_factor = Some(f);
                                p.save();
                            }
                        }
                        None => {}
                    }
                } else if self.overlay == Overlay::Stats {
                    if crate::ui::stats_panel::show(ui, &mut kit, &self.store) {
                        self.overlay = Overlay::None;
                    }
                } else if self.overlay == Overlay::Help {
                    let mode = self.store.editor.mode;
                    if self.help.show(ui, &mut kit, mode) {
                        self.overlay = Overlay::None;
                    }
                } else {
                    if self.store.editor.mode != EditorMode::Pan {
                        egui::Panel::left("side-panel")
                            .exact_size(theme::SIDE_PANEL_WIDTH)
                            .resizable(false)
                            .frame(
                                egui::Frame::new()
                                    .fill(t.bg_panel)
                                    .inner_margin(16)
                                    .stroke(egui::Stroke::new(1.0, t.border)),
                            )
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    match self.store.editor.mode {
                                        EditorMode::Pin => pin_panel(ui, &mut kit, &mut self.store),
                                        EditorMode::Select => {
                                            select_panel(ui, &mut kit, &mut self.store)
                                        }
                                        EditorMode::Thread => {
                                            thread_panel(ui, &mut kit, &mut self.store)
                                        }
                                        EditorMode::Play => {
                                            self.play.panel(ui, &mut kit, &self.store)
                                        }
                                        EditorMode::Generate => {
                                            self.generator.show(ui, &mut kit, &mut self.store)
                                        }
                                        mode => {
                                            let title =
                                                kit.tr(&format!("common:modes.{}", mode.key()));
                                            shell::pending_panel(ui, &kit, &title);
                                        }
                                    }
                                });
                            });
                    }
                    egui::Panel::right("layers-panel")
                        .exact_size(theme::LAYERS_PANEL_WIDTH)
                        .resizable(false)
                        .frame(
                            egui::Frame::new()
                                .fill(t.bg_panel)
                                .inner_margin(16)
                                .stroke(egui::Stroke::new(1.0, t.border)),
                        )
                        .show(ui, |ui| {
                            self.layers_panel.show(ui, &mut kit, &mut self.store);
                        });
                    egui::CentralPanel::no_frame().show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        if self.store.editor.mode == EditorMode::Play {
                            self.canvas.show_playback(
                                ui,
                                &kit,
                                &self.store,
                                self.play.transport.frame,
                            );
                            return;
                        }
                        let pin_tool = &mut self.pin_tool;
                        let select_tool = &mut self.select_tool;
                        let thread_tool = &mut self.thread_tool;
                        self.canvas.show(
                            ui,
                            &mut kit,
                            &mut self.store,
                            |ui, kit, store: &mut Store, input| match store.editor.mode {
                                EditorMode::Pin => pin_tool.frame(ui, kit, store, input),
                                EditorMode::Select => {
                                    select_tool.frame(ui, kit, store, input);
                                    None
                                }
                                EditorMode::Thread => thread_tool.frame(ui, kit, store, input),
                                EditorMode::Generate => {
                                    let painter = ui.painter_at(input.response.rect);
                                    crate::ui::generator_panel::paint_preview(
                                        &painter, &input.xf, store,
                                    );
                                    None
                                }
                                _ => None,
                            },
                        );
                    });
                }
                if let Some(a) = action {
                    self.handle_shell_action(ui.ctx(), a);
                }
            }
        }

        if self.screen == Screen::Editor && self.overlay == Overlay::None {
            // Radial context menu (spec 25).
            let mut kit = Kit {
                t,
                icons: &mut self.icons,
                i18n: self.i18n,
            };
            let was_open = self.radial.is_open();
            let playing = self.play.transport.playing;
            if let Some(cmd) =
                self.radial
                    .show(ui, &mut kit, &mut self.store, &mut self.canvas, playing)
            {
                let now = ui.input(|i| i.time);
                self.play.command(cmd, &self.store, now);
            }
            if !was_open {
                self.radial.maybe_open(ui, &self.store, self.canvas.rect());
            }
            // Play mode: transport tick + keys; entering Play resets to frame 0.
            if self.store.editor.mode == EditorMode::Play {
                if self.last_mode != EditorMode::Play {
                    self.play.transport.reset();
                }
                let kit = Kit {
                    t,
                    icons: &mut self.icons,
                    i18n: self.i18n,
                };
                self.play.tick(ui.ctx(), &self.store, &kit);
                if !self.radial.is_open() {
                    self.play.keys(ui.ctx(), &self.store, &kit);
                }
            }
            self.last_mode = self.store.editor.mode;
            // Keyboard shortcuts (spec 34).
            if !self.radial.is_open()
                && let Some(a) =
                    crate::shortcuts::dispatch(ui.ctx(), &mut self.store, &mut self.canvas)
            {
                self.handle_shell_action(ui.ctx(), a);
            }
        }

        self.session_frame(ui.ctx());
    }
}
