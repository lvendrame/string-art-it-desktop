//! Local MCP agent server lifecycle (spec 31 desktop). The server runs on its
//! own thread; tool calls are executed here, on the UI thread, against the
//! live store so the canvas, undo history and dirty tracking all follow.

use std::sync::mpsc::Receiver;

use sa_app::editor::Store;
use sa_infra::mcp::{McpServer, ToolCall};

#[derive(Default)]
pub struct Agent {
    server: Option<(McpServer, Receiver<ToolCall>)>,
    pub error: Option<String>,
}

/// What the top-bar popover shows.
pub struct AgentStatus {
    pub enabled: bool,
    pub url: Option<String>,
    pub error: Option<String>,
}

impl Agent {
    pub fn status(&self) -> AgentStatus {
        AgentStatus {
            enabled: self.server.is_some(),
            url: self.server.as_ref().map(|(s, _)| s.url()),
            error: self.error.clone(),
        }
    }

    pub fn set_enabled(&mut self, ctx: &egui::Context, on: bool, port: u16) {
        self.error = None;
        if !on {
            if let Some((server, _)) = self.server.take() {
                server.stop();
            }
            return;
        }
        if self.server.is_some() {
            return;
        }
        let repaint = ctx.clone();
        match McpServer::start(port, move || repaint.request_repaint()) {
            Ok(s) => self.server = Some(s),
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Runs every queued tool call. Returns true when any ran.
    pub fn pump(&mut self, store: &mut Store) -> bool {
        let Some((_, rx)) = &self.server else {
            return false;
        };
        let mut any = false;
        while let Ok(call) = rx.try_recv() {
            let result = sa_app::agent_tools::execute(store, &call.name, &call.args);
            log::info!(
                "agent tool {} -> {}",
                call.name,
                if result.is_ok() { "ok" } else { "error" }
            );
            let _ = call.reply.send(result);
            any = true;
        }
        any
    }
}
