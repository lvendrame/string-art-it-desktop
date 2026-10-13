//! Local MCP server (spec 31, desktop replacement for WebMCP). Serves the
//! `sa_app::agent_tools` over Streamable HTTP on 127.0.0.1 from a background
//! tokio runtime. Tool calls are forwarded to the UI thread (which owns the
//! `Store`) through a channel and answered there, so agents drive the live,
//! visible document with normal undo.
//!
//! Security: loopback bind, rmcp's loopback `Host` allow-list (DNS-rebinding
//! guard) and an `Origin` allow-list, so web pages can't call it.

use std::sync::Arc;
use std::sync::mpsc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    InitializeResult, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig,
    Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use serde_json::Value;

pub const DEFAULT_PORT: u16 = 47_821;

/// One tool invocation awaiting the UI thread.
pub struct ToolCall {
    pub name: String,
    pub args: Value,
    pub reply: tokio::sync::oneshot::Sender<Result<Value, String>>,
}

type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
struct Handler {
    tx: mpsc::Sender<ToolCall>,
    wake: Wake,
    tools: Arc<Vec<Tool>>,
}

impl ServerHandler for Handler {
    fn get_info(&self) -> ServerConfig {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("string-art-it", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Drives the StringArtIt document open in the desktop app: board setup, pin/thread layers and paths, undo/redo. Coordinates are centimetres with the board centre at (0, 0).",
            )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tools.as_ref().clone()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        let call = ToolCall {
            name: request.name.into_owned(),
            args: Value::Object(request.arguments.unwrap_or_default()),
            reply,
        };
        if self.tx.send(call).is_err() {
            return Err(rmcp::ErrorData::internal_error(
                "app is shutting down",
                None,
            ));
        }
        (self.wake)();
        let result = match rx.await {
            Ok(Ok(v)) => CallToolResult::structured(v),
            Ok(Err(msg)) => CallToolResult::error(vec![ContentBlock::text(msg)]),
            Err(_) => return Err(rmcp::ErrorData::internal_error("tool call dropped", None)),
        };
        Ok(result.into())
    }
}

fn tool_list() -> Vec<Tool> {
    sa_app::agent_tools::tools()
        .into_iter()
        .map(|t| {
            let schema = match t.input_schema {
                Value::Object(m) => m,
                _ => Default::default(),
            };
            Tool::new(t.name, t.description, Arc::new(schema))
        })
        .collect()
}

/// A running server; dropping it (or `stop`) shuts it down.
pub struct McpServer {
    pub port: u16,
    stop: Option<Box<dyn FnOnce() + Send>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl McpServer {
    /// Binds `127.0.0.1:port` (0 = any free port) and starts serving.
    /// `wake` is called after each queued tool call (e.g. `ctx.request_repaint`).
    pub fn start(
        port: u16,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> std::io::Result<(McpServer, mpsc::Receiver<ToolCall>)> {
        let listener = std::net::TcpListener::bind(("127.0.0.1", port))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let (tx, rx) = mpsc::channel();
        let handler = Handler {
            tx,
            wake: Arc::new(wake),
            tools: Arc::new(tool_list()),
        };
        let mut config = StreamableHttpServerConfig::default();
        config.allowed_origins = vec![
            format!("http://127.0.0.1:{port}"),
            format!("http://localhost:{port}"),
        ];
        let token = config.cancellation_token.clone();
        let shutdown = token.clone();
        let service = StreamableHttpService::new(
            move || Ok(handler.clone()),
            Arc::new(LocalSessionManager::default()),
            config,
        );
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("mcp")
            .enable_all()
            .build()?;
        let thread = std::thread::Builder::new()
            .name("mcp-server".into())
            .spawn(move || {
                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::from_std(listener) {
                        Ok(l) => l,
                        Err(e) => return log::error!("mcp listener: {e}"),
                    };
                    let router = axum::Router::new().nest_service("/mcp", service);
                    if let Err(e) = axum::serve(listener, router)
                        .with_graceful_shutdown(shutdown.cancelled_owned())
                        .await
                    {
                        log::error!("mcp server: {e}");
                    }
                });
            })?;
        log::info!("MCP server listening on http://127.0.0.1:{port}/mcp");
        Ok((
            McpServer {
                port,
                stop: Some(Box::new(move || token.cancel())),
                thread: Some(thread),
            },
            rx,
        ))
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop();
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    /// Minimal HTTP/1.1 JSON-RPC POST; returns (session id, JSON body from JSON or SSE).
    fn post(
        port: u16,
        session: Option<&str>,
        origin: Option<&str>,
        body: &Value,
    ) -> (u16, Option<String>, Option<Value>) {
        let body = body.to_string();
        let mut req = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2025-06-18\r\nContent-Length: {}\r\nConnection: close\r\n",
            body.len()
        );
        if let Some(s) = session {
            req.push_str(&format!("Mcp-Session-Id: {s}\r\n"));
        }
        if let Some(o) = origin {
            req.push_str(&format!("Origin: {o}\r\n"));
        }
        req.push_str("\r\n");
        req.push_str(&body);
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        stream.write_all(req.as_bytes()).unwrap();
        let mut raw = Vec::new();
        // SSE streams may stay open; read until the first complete JSON data event.
        let mut buf = [0u8; 4096];
        loop {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => raw.extend_from_slice(&buf[..n]),
            }
            let text = String::from_utf8_lossy(&raw);
            let complete = text
                .lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .any(|d| serde_json::from_str::<Value>(d.trim()).is_ok());
            if complete {
                break;
            }
        }
        let text = String::from_utf8_lossy(&raw).to_string();
        let (head, payload) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let sid = head.lines().find_map(|l| {
            l.to_ascii_lowercase()
                .starts_with("mcp-session-id:")
                .then(|| l[15..].trim().to_owned())
        });
        let json = payload
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .find_map(|d| serde_json::from_str(d.trim()).ok())
            .or_else(|| {
                // plain JSON (possibly chunked: take the first {...} span)
                let start = payload.find('{')?;
                let end = payload.rfind('}')?;
                serde_json::from_str(&payload[start..=end]).ok()
            });
        (status, sid, json)
    }

    #[test]
    fn end_to_end_tool_call() {
        let (server, rx) = McpServer::start(0, || {}).unwrap();
        let port = server.port;
        // "UI thread": owns the store and answers calls.
        let ui = std::thread::spawn(move || {
            let mut store = sa_app::editor::Store::default();
            let mut served = 0;
            while let Ok(call) = rx.recv() {
                let _ = call.reply.send(sa_app::agent_tools::execute(
                    &mut store, &call.name, &call.args,
                ));
                served += 1;
            }
            (served, store.doc.board.dimensions.diameter)
        });

        let init = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }
        });
        let (status, sid, body) = post(port, None, None, &init);
        assert_eq!(status, 200, "{body:?}");
        let sid = sid.expect("session id");
        assert_eq!(
            body.unwrap()["result"]["serverInfo"]["name"],
            "string-art-it"
        );
        post(
            port,
            Some(&sid),
            None,
            &serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        );

        let (_, _, list) = post(
            port,
            Some(&sid),
            None,
            &serde_json::json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
        );
        let tools = list.unwrap()["result"]["tools"].as_array().unwrap().len();
        assert_eq!(tools, 18);

        let call = serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "set_board_dimensions", "arguments": { "diameter": 45 } }
        });
        let (_, _, res) = post(port, Some(&sid), None, &call);
        let res = res.unwrap();
        assert_eq!(
            res["result"]["structuredContent"]["board"]["dimensions"]["diameter"], 45.0,
            "{res}"
        );

        let bad = serde_json::json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "nope", "arguments": {} } });
        let (_, _, res) = post(port, Some(&sid), None, &bad);
        assert_eq!(res.unwrap()["result"]["isError"], true);

        // A web page's Origin is refused.
        let (status, _, _) = post(port, None, Some("https://evil.example"), &init);
        assert_eq!(status, 403);

        server.stop();
        let (served, diameter) = ui.join().unwrap();
        assert_eq!(served, 2);
        assert_eq!(diameter, Some(45.0));
    }
}
