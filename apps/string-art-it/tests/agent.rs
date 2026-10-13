//! MCP agent server driving the live app (spec 31 desktop).

use std::io::{Read, Write};
use std::sync::mpsc;

use egui_kittest::Harness;
use serde_json::{Value, json};
use string_art_it::app::StringArtApp;

/// JSON-RPC POST over raw HTTP/1.1; returns (session id, first JSON data event).
fn post(port: u16, session: Option<&str>, body: &Value) -> (Option<String>, Option<Value>) {
    let body = body.to_string();
    let mut req = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2025-06-18\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(s) = session {
        req.push_str(&format!("Mcp-Session-Id: {s}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(&body);
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    stream.write_all(req.as_bytes()).unwrap();
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    let parse = |raw: &[u8]| -> Option<Value> {
        String::from_utf8_lossy(raw)
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .find_map(|d| serde_json::from_str(d.trim()).ok())
    };
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => raw.extend_from_slice(&buf[..n]),
        }
        if parse(&raw).is_some() {
            break;
        }
    }
    let text = String::from_utf8_lossy(&raw).to_string();
    let sid = text.lines().find_map(|l| {
        l.to_ascii_lowercase()
            .starts_with("mcp-session-id:")
            .then(|| l[15..].trim().to_owned())
    });
    (sid, parse(&raw))
}

#[test]
fn agent_edits_live_document() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_eframe(|cc| StringArtApp::with_context(&cc.egui_ctx));
    let ctx = h.ctx.clone();
    let url = h
        .state_mut()
        .start_agent_for_test(&ctx, 0)
        .expect("server started");
    let port: u16 = url
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches("/mcp")
        .parse()
        .unwrap();
    h.state_mut().enter_editor();
    h.run_steps(2);

    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (sid, _) = post(
            port,
            None,
            &json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }
            }),
        );
        let sid = sid.unwrap();
        post(
            port,
            Some(&sid),
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        );
        let (_, stats) = post(
            port,
            Some(&sid),
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "get_document_stats", "arguments": {} } }),
        );
        let layer = stats.unwrap()["result"]["structuredContent"]["pinLayers"][0]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (_, added) = post(
            port,
            Some(&sid),
            &json!({
                "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": { "name": "add_pin_path", "arguments": { "layerId": layer, "geometry": { "type": "circle", "center": { "x": 0, "y": 0 }, "radius": 12 } } }
            }),
        );
        done_tx.send(added.unwrap()).unwrap();
    });

    // Pump frames (the UI thread executes queued calls) until the client is done.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let added = loop {
        h.run_steps(1);
        if let Ok(v) = done_rx.try_recv() {
            break v;
        }
        assert!(std::time::Instant::now() < deadline, "agent call timed out");
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    assert_eq!(
        added["result"]["structuredContent"]["success"], true,
        "{added}"
    );
    let store = h.state_mut().store_mut();
    assert_eq!(store.doc.pin_layers[0].pin_paths.len(), 1);
    assert!(store.can_undo());
}
