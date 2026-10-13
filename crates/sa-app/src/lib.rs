//! Application layer: document model, project file, undo history, editor
//! state and the algorithms behind each tool. No UI, no IO — port of web
//! `src/application`.

pub mod agent_tools;
pub mod clipboard;
pub mod document;
pub mod editor;
pub mod editor_pins;
pub mod editor_select;
pub mod editor_threads;
pub mod generator;
pub mod history;
pub mod hit;
pub mod ids;
pub mod pin_tools;
pub mod playback;
pub mod print;
pub mod statistics;
pub mod threads;

pub use document::Document;
