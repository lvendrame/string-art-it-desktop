//! Agent tools (spec 31, desktop: served over MCP instead of WebMCP). Same 18
//! tools, names, input schemas and result shapes as web `webmcp/tools.ts`;
//! every mutation reuses an existing `Store` method, so undo / locked-layer
//! behaviour is inherited unchanged.

use serde_json::{Value, json};

use crate::document::layers::{find_pin_by_id, is_layer_locked};
use crate::document::{
    BoardAppearance, BoardDimensions, BoardShape, PinPathGeometry, TriangleType,
};
use crate::editor::Store;
use crate::statistics::{project_thread_totals, project_total_pins};

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
}

fn point() -> Value {
    json!({ "type": "object", "properties": { "x": { "type": "number" }, "y": { "type": "number" } }, "required": ["x", "y"] })
}

fn geometry_schema() -> Value {
    let p = point;
    json!({
        "description": "A Pin Path shape in the board's physical coordinate space (centimetres). `text` geometry is not supported here — it requires font-loading infra that stays UI-only.",
        "oneOf": [
            { "type": "object", "properties": { "type": { "const": "line" }, "start": p(), "end": p() }, "required": ["type", "start", "end"] },
            { "type": "object", "properties": { "type": { "const": "arc" }, "start": p(), "end": p(), "curvature": { "type": "number" } }, "required": ["type", "start", "end", "curvature"] },
            { "type": "object", "properties": { "type": { "const": "ellipse" }, "center": p(), "radiusX": { "type": "number" }, "radiusY": { "type": "number" }, "rotation": { "type": "number" } }, "required": ["type", "center", "radiusX", "radiusY", "rotation"] },
            { "type": "object", "properties": { "type": { "const": "circle" }, "center": p(), "radius": { "type": "number" } }, "required": ["type", "center", "radius"] },
            { "type": "object", "properties": { "type": { "const": "rectangle" }, "position": p(), "width": { "type": "number" }, "height": { "type": "number" }, "rotation": { "type": "number" } }, "required": ["type", "position", "width", "height", "rotation"] },
            { "type": "object", "properties": { "type": { "const": "square" }, "position": p(), "side": { "type": "number" }, "rotation": { "type": "number" } }, "required": ["type", "position", "side", "rotation"] },
            { "type": "object", "properties": { "type": { "const": "regular-polygon" }, "center": p(), "radius": { "type": "number" }, "sides": { "type": "number" }, "rotation": { "type": "number" } }, "required": ["type", "center", "radius", "sides", "rotation"] },
            { "type": "object", "properties": { "type": { "const": "star" }, "center": p(), "outerRadius": { "type": "number" }, "innerRadius": { "type": "number" }, "points": { "type": "number" }, "rotation": { "type": "number" } }, "required": ["type", "center", "outerRadius", "innerRadius", "points", "rotation"] },
            { "type": "object", "properties": { "type": { "const": "polygram" }, "center": p(), "radius": { "type": "number" }, "points": { "type": "number" }, "skip": { "type": "number" }, "rotation": { "type": "number" } }, "required": ["type", "center", "radius", "points", "skip", "rotation"] },
            { "type": "object", "properties": { "type": { "const": "freehand" }, "points": { "type": "array", "items": p() } }, "required": ["type", "points"] }
        ]
    })
}

fn empty() -> Value {
    json!({ "type": "object", "properties": {} })
}

fn layer_id() -> Value {
    json!({ "type": "object", "properties": { "layerId": { "type": "string" } }, "required": ["layerId"] })
}

fn layer_path() -> Value {
    json!({ "type": "object", "properties": { "layerId": { "type": "string" }, "pathId": { "type": "string" } }, "required": ["layerId", "pathId"] })
}

pub fn tools() -> Vec<ToolDef> {
    let t = |name, description, input_schema| ToolDef {
        name,
        description,
        input_schema,
    };
    vec![
        t(
            "get_document_stats",
            "Read the current document's board setup, per-layer summaries, pin/thread totals, and undo/redo availability.",
            empty(),
        ),
        t(
            "set_board_shape",
            "Set the board's shape, optionally its triangle type.",
            json!({
                "type": "object",
                "properties": {
                    "shape": { "type": "string", "enum": ["circle", "oval", "rectangle", "square", "triangle"] },
                    "triangleType": { "type": "string", "enum": ["equilateral", "right-angled"] }
                },
                "required": ["shape"]
            }),
        ),
        t(
            "set_board_dimensions",
            "Set one or more of the board's physical dimensions (centimetres): diameter, width, height, side, base — whichever apply to the current shape.",
            json!({
                "type": "object",
                "properties": {
                    "diameter": { "type": "number" }, "width": { "type": "number" }, "height": { "type": "number" },
                    "side": { "type": "number" }, "base": { "type": "number" }
                }
            }),
        ),
        t(
            "set_board_appearance",
            "Set the board's appearance to a solid colour or a named wood/painted preset. Gradients and custom-texture uploads are not supported here — use the UI for those.",
            json!({
                "type": "object",
                "oneOf": [
                    { "properties": { "type": { "const": "solid" }, "colour": { "type": "string" } }, "required": ["type", "colour"] },
                    { "properties": { "type": { "enum": ["wood-texture", "painted-wood"] }, "presetId": { "type": "string" } }, "required": ["type", "presetId"] }
                ]
            }),
        ),
        t(
            "add_pin_layer",
            "Create a new Pin Layer and make it the active one.",
            empty(),
        ),
        t(
            "set_active_pin_layer",
            "Make an existing Pin Layer the active one (new Pin Paths are added to whichever layer is active).",
            layer_id(),
        ),
        t(
            "toggle_pin_layer_visible",
            "Toggle a Pin Layer's visibility.",
            layer_id(),
        ),
        t(
            "toggle_pin_layer_locked",
            "Toggle a Pin Layer's locked state (locked layers block edits).",
            layer_id(),
        ),
        t(
            "add_pin_path",
            "Draw a new Pin Path on a Pin Layer from a geometry description. Pins are distributed automatically using the document's current spacing default.",
            json!({
                "type": "object",
                "properties": { "layerId": { "type": "string" }, "geometry": geometry_schema() },
                "required": ["layerId", "geometry"]
            }),
        ),
        t(
            "delete_pin_path",
            "Delete a Pin Path (and any Thread Path segments attached to its pins) from a Pin Layer.",
            layer_path(),
        ),
        t(
            "add_thread_layer",
            "Create a new Thread Layer and make it the active one.",
            empty(),
        ),
        t(
            "set_active_thread_layer",
            "Make an existing Thread Layer the active one (new Thread Paths are added to whichever layer is active).",
            layer_id(),
        ),
        t(
            "toggle_thread_layer_visible",
            "Toggle a Thread Layer's visibility.",
            layer_id(),
        ),
        t(
            "toggle_thread_layer_locked",
            "Toggle a Thread Layer's locked state (locked layers block edits).",
            layer_id(),
        ),
        t(
            "add_thread_path",
            "Draw a new Thread Path connecting 2 or more existing pins (by id, in order) on a Thread Layer. Composes the same primitives the click-to-draw UI uses.",
            json!({
                "type": "object",
                "properties": { "layerId": { "type": "string" }, "pinIds": { "type": "array", "items": { "type": "string" }, "minItems": 2 } },
                "required": ["layerId", "pinIds"]
            }),
        ),
        t(
            "delete_thread_path",
            "Delete a Thread Path from a Thread Layer.",
            layer_path(),
        ),
        t("undo", "Undo the last document change.", empty()),
        t("redo", "Redo the last undone document change.", empty()),
    ]
}

fn layer_summary(store: &Store) -> (Value, Value) {
    let pin = store
        .doc
        .pin_layers
        .iter()
        .map(|l| {
            json!({
                "id": l.id, "name": l.name, "visible": l.visible, "locked": l.locked,
                "pathCount": l.pin_paths.len(),
                "pinCount": l.pin_paths.iter().map(|p| p.pins.len()).sum::<usize>(),
            })
        })
        .collect();
    let thread = store
        .doc
        .thread_layers
        .iter()
        .map(|l| json!({ "id": l.id, "name": l.name, "visible": l.visible, "locked": l.locked, "pathCount": l.thread_paths.len() }))
        .collect();
    (Value::Array(pin), Value::Array(thread))
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string argument `{key}`"))
}

fn parse<T: serde::de::DeserializeOwned>(v: Value, what: &str) -> Result<T, String> {
    serde_json::from_value(v).map_err(|e| format!("invalid {what}: {e}"))
}

fn history(store: &Store) -> Value {
    json!({ "canUndo": store.can_undo(), "canRedo": store.can_redo() })
}

/// Runs one tool. `Err` is an invalid call (unknown tool / malformed input);
/// domain refusals (locked layer, …) are `Ok({"success": false, …})` like the web.
pub fn execute(store: &mut Store, name: &str, args: &Value) -> Result<Value, String> {
    let board = |s: &Store| json!({ "board": s.doc.board });
    match name {
        "get_document_stats" => {
            let (pin_layers, thread_layers) = layer_summary(store);
            let totals = project_thread_totals(&store.doc.thread_layers, &store.doc.pin_layers);
            Ok(json!({
                "board": store.doc.board,
                "pinLayers": pin_layers,
                "threadLayers": thread_layers,
                "totalPins": project_total_pins(&store.doc.pin_layers),
                "threadCount": totals.thread_count,
                "totalLengthCm": totals.total_length_cm,
                "canUndo": store.can_undo(),
                "canRedo": store.can_redo(),
            }))
        }
        "set_board_shape" => {
            let shape: BoardShape =
                parse(args.get("shape").cloned().unwrap_or(Value::Null), "shape")?;
            let tri: Option<TriangleType> = match args.get("triangleType") {
                None | Some(Value::Null) => None,
                Some(v) => Some(parse(v.clone(), "triangleType")?),
            };
            store.set_board_shape(shape, tri);
            Ok(board(store))
        }
        "set_board_dimensions" => {
            let patch: BoardDimensions = parse(args.clone(), "dimensions")?;
            store.set_board_dimensions(patch);
            Ok(board(store))
        }
        "set_board_appearance" => {
            let appearance: BoardAppearance = parse(args.clone(), "appearance")?;
            if !matches!(
                appearance,
                BoardAppearance::Solid { .. }
                    | BoardAppearance::WoodTexture { .. }
                    | BoardAppearance::PaintedWood { .. }
            ) {
                return Err(
                    "only `solid`, `wood-texture` and `painted-wood` appearances are supported"
                        .into(),
                );
            }
            store.set_board_appearance(appearance);
            Ok(board(store))
        }
        "add_pin_layer" => {
            store.add_pin_layer();
            Ok(json!({ "layerId": store.editor.active_pin_layer_id }))
        }
        "set_active_pin_layer" => {
            let id = str_arg(args, "layerId")?;
            if store.doc.pin_layers.iter().any(|l| l.id == id) {
                store.editor.active_pin_layer_id = id.to_owned();
            }
            Ok(json!({ "activePinLayerId": store.editor.active_pin_layer_id }))
        }
        "toggle_pin_layer_visible" | "toggle_pin_layer_locked" => {
            let id = str_arg(args, "layerId")?.to_owned();
            if name.ends_with("visible") {
                store.toggle_pin_layer_visible(&id);
            } else {
                store.toggle_pin_layer_locked(&id);
            }
            Ok(json!({ "pinLayers": layer_summary(store).0 }))
        }
        "add_pin_path" => {
            let layer = str_arg(args, "layerId")?.to_owned();
            let geometry: PinPathGeometry = parse(
                args.get("geometry").cloned().unwrap_or(Value::Null),
                "geometry",
            )?;
            if matches!(geometry, PinPathGeometry::Text { .. }) {
                return Err("`text` geometry is not supported by this tool".into());
            }
            if is_layer_locked(&store.doc.pin_layers, &layer) {
                return Ok(json!({ "success": false, "reason": "layer_locked" }));
            }
            Ok(match store.add_pin_path(&layer, geometry) {
                Some(path_id) => json!({ "success": true, "pathId": path_id }),
                None => json!({ "success": false, "reason": "layer_locked" }),
            })
        }
        "delete_pin_path" => {
            let (layer, path) = (
                str_arg(args, "layerId")?.to_owned(),
                str_arg(args, "pathId")?.to_owned(),
            );
            store.delete_pin_path(&layer, &path);
            Ok(json!({ "success": true }))
        }
        "add_thread_layer" => {
            store.add_thread_layer();
            Ok(json!({ "layerId": store.editor.active_thread_layer_id }))
        }
        "set_active_thread_layer" => {
            let id = str_arg(args, "layerId")?;
            if store.doc.thread_layers.iter().any(|l| l.id == id) {
                store.editor.active_thread_layer_id = id.to_owned();
            }
            Ok(json!({ "activeThreadLayerId": store.editor.active_thread_layer_id }))
        }
        "toggle_thread_layer_visible" | "toggle_thread_layer_locked" => {
            let id = str_arg(args, "layerId")?.to_owned();
            if name.ends_with("visible") {
                store.toggle_thread_layer_visible(&id);
            } else {
                store.toggle_thread_layer_locked(&id);
            }
            Ok(json!({ "threadLayers": layer_summary(store).1 }))
        }
        "add_thread_path" => {
            let layer = str_arg(args, "layerId")?.to_owned();
            let pin_ids: Vec<String> =
                parse(args.get("pinIds").cloned().unwrap_or(Value::Null), "pinIds")?;
            if is_layer_locked(&store.doc.thread_layers, &layer) {
                return Ok(json!({ "success": false, "reason": "layer_locked" }));
            }
            if pin_ids.len() < 2 {
                return Ok(json!({ "success": false, "reason": "too_few_pins" }));
            }
            let ids_of = |s: &Store| -> Vec<String> {
                s.doc
                    .thread_layers
                    .iter()
                    .filter(|l| l.id == layer)
                    .flat_map(|l| l.thread_paths.iter().map(|t| t.id.clone()))
                    .collect()
            };
            let before = ids_of(store);
            store.cancel_thread_draft();
            let valid: Vec<&String> = pin_ids
                .iter()
                .filter(|id| find_pin_by_id(&store.doc.pin_layers, id).is_some())
                .collect();
            for id in valid {
                store.extend_thread_draft(id);
            }
            store.finish_thread_draft(&layer);
            Ok(
                match ids_of(store).into_iter().find(|id| !before.contains(id)) {
                    Some(path_id) => json!({ "success": true, "pathId": path_id }),
                    None => json!({ "success": false, "reason": "not_created" }),
                },
            )
        }
        "delete_thread_path" => {
            let (layer, path) = (
                str_arg(args, "layerId")?.to_owned(),
                str_arg(args, "pathId")?.to_owned(),
            );
            store.delete_thread_path(&layer, &path);
            Ok(json!({ "success": true }))
        }
        "undo" => {
            store.undo();
            Ok(history(store))
        }
        "redo" => {
            store.redo();
            Ok(history(store))
        }
        other => Err(format!("unknown tool `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle(store: &mut Store, layer: &str) -> Value {
        execute(
            store,
            "add_pin_path",
            &json!({ "layerId": layer, "geometry": { "type": "circle", "center": { "x": 0, "y": 0 }, "radius": 10 } }),
        )
        .unwrap()
    }

    #[test]
    fn registry_has_every_spec_tool_once() {
        let names: Vec<_> = tools().iter().map(|t| t.name).collect();
        assert_eq!(names.len(), 18);
        let mut dedup = names.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(dedup.len(), 18);
        let mut s = Store::default();
        for n in names {
            // every registered tool dispatches (errors only for missing args)
            let r = execute(&mut s, n, &json!({}));
            assert!(
                r.is_ok() || r.unwrap_err().contains("argument") || n.starts_with("set_board"),
                "{n}"
            );
        }
    }

    #[test]
    fn stats_reflect_live_state() {
        let mut s = Store::default();
        let layer = s.editor.active_pin_layer_id.clone();
        circle(&mut s, &layer);
        execute(&mut s, "add_pin_layer", &json!({})).unwrap();
        let v = execute(&mut s, "get_document_stats", &json!({})).unwrap();
        assert_eq!(v["pinLayers"].as_array().unwrap().len(), 2);
        assert_eq!(
            v["totalPins"],
            json!(s.doc.pin_layers[0].pin_paths[0].pins.len())
        );
        assert_eq!(v["board"]["shape"], json!("circle"));
    }

    #[test]
    fn dimensions_are_undoable() {
        let mut s = Store::default();
        let before = s.doc.board.dimensions.diameter;
        let v = execute(&mut s, "set_board_dimensions", &json!({ "diameter": 45 })).unwrap();
        assert_eq!(v["board"]["dimensions"]["diameter"], json!(45.0));
        let h = execute(&mut s, "undo", &json!({})).unwrap();
        assert_eq!(h, json!({ "canUndo": false, "canRedo": true }));
        assert_eq!(s.doc.board.dimensions.diameter, before);
    }

    #[test]
    fn locked_layer_fails_structurally() {
        let mut s = Store::default();
        let layer = s.editor.active_pin_layer_id.clone();
        execute(
            &mut s,
            "toggle_pin_layer_locked",
            &json!({ "layerId": layer }),
        )
        .unwrap();
        assert_eq!(
            circle(&mut s, &layer),
            json!({ "success": false, "reason": "layer_locked" })
        );
        assert!(s.doc.pin_layers[0].pin_paths.is_empty());
    }

    #[test]
    fn thread_path_is_one_undo_step() {
        let mut s = Store::default();
        let layer = s.editor.active_pin_layer_id.clone();
        let p = circle(&mut s, &layer);
        assert_eq!(p["success"], json!(true));
        let pins: Vec<String> = s.doc.pin_layers[0].pin_paths[0]
            .pins
            .iter()
            .take(4)
            .map(|p| p.id.clone())
            .collect();
        let tl = s.editor.active_thread_layer_id.clone();
        let r = execute(
            &mut s,
            "add_thread_path",
            &json!({ "layerId": tl, "pinIds": pins }),
        )
        .unwrap();
        assert_eq!(r["success"], json!(true));
        assert_eq!(s.doc.thread_layers[0].thread_paths[0].pin_ids, pins);
        s.undo();
        assert!(s.doc.thread_layers[0].thread_paths.is_empty());
        let r = execute(
            &mut s,
            "add_thread_path",
            &json!({ "layerId": tl, "pinIds": ["only-one"] }),
        )
        .unwrap();
        assert_eq!(r["reason"], json!("too_few_pins"));
    }

    #[test]
    fn rejects_text_geometry_and_unsupported_appearance() {
        let mut s = Store::default();
        let r = execute(
            &mut s,
            "set_board_appearance",
            &json!({ "type": "linear-gradient", "stops": [], "direction": 0 }),
        );
        assert!(r.is_err());
        let r = execute(
            &mut s,
            "set_board_appearance",
            &json!({ "type": "solid", "colour": "#123456" }),
        )
        .unwrap();
        assert_eq!(r["board"]["appearance"]["colour"], json!("#123456"));
        assert!(execute(&mut s, "nope", &json!({})).is_err());
    }
}
