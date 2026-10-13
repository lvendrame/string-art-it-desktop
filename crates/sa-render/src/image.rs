//! Custom board textures stored as data URLs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use base64::Engine;

/// Straight-alpha RGBA8 pixels.
#[derive(Debug)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Original encoded bytes + mime, so vector exports can embed without re-encoding.
    pub encoded: Vec<u8>,
    pub mime: String,
}

/// Decodes (and caches) a `data:image/...;base64,...` URL.
pub fn decode_data_url(url: &str) -> Option<Arc<DecodedImage>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, Arc<DecodedImage>>>> = OnceLock::new();
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        url.hash(&mut h);
        h.finish()
    };
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().ok()?.get(&key) {
        return Some(hit.clone());
    }
    let rest = url.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    let mime = meta.split(';').next().unwrap_or("").to_owned();
    let encoded = if meta.ends_with(";base64") {
        base64::engine::general_purpose::STANDARD
            .decode(payload.trim())
            .ok()?
    } else {
        payload.as_bytes().to_vec()
    };
    let img = image::load_from_memory(&encoded).ok()?.to_rgba8();
    let decoded = Arc::new(DecodedImage {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
        encoded,
        mime,
    });
    cache.lock().ok()?.insert(key, decoded.clone());
    Some(decoded)
}
