//! Play mode frame model (spec 19). Port of web `playback.ts`: a frame is one
//! thread segment, ordered by thread layer → path → pinIds.

use crate::Document;
use crate::document::ThreadLayer;

pub fn total_thread_frames(layers: &[ThreadLayer]) -> usize {
    layers
        .iter()
        .flat_map(|l| &l.thread_paths)
        .map(|p| p.segment_count())
        .sum()
}

/// The first `frame` segments only (paths beyond the budget dropped, the
/// straddling path sliced).
pub fn truncate_at_frame(layers: &[ThreadLayer], frame: usize) -> Vec<ThreadLayer> {
    let mut remaining = frame;
    layers
        .iter()
        .map(|l| {
            let mut paths = Vec::new();
            for p in &l.thread_paths {
                if remaining == 0 {
                    break;
                }
                let seg = p.segment_count();
                if remaining >= seg {
                    paths.push(p.clone());
                    remaining -= seg;
                } else {
                    let mut q = p.clone();
                    q.pin_ids.truncate(remaining + 1);
                    paths.push(q);
                    remaining = 0;
                }
            }
            ThreadLayer {
                thread_paths: paths,
                ..l.clone()
            }
        })
        .collect()
}

/// Document snapshot showing frame `frame`.
pub fn document_at_frame(doc: &Document, frame: usize) -> Document {
    Document {
        thread_layers: truncate_at_frame(&doc.thread_layers, frame),
        ..doc.clone()
    }
}

/// Play/pause/step transport (web `usePlaybackTransport`); time in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Transport {
    pub frame: usize,
    pub playing: bool,
    pub interval_ms: f64,
    next_tick: f64,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            frame: 0,
            playing: false,
            interval_ms: 150.0,
            next_tick: 0.0,
        }
    }
}

impl Transport {
    /// Entering Play mode resets to frame 0, paused.
    pub fn reset(&mut self) {
        self.frame = 0;
        self.playing = false;
    }

    pub fn go_to(&mut self, frame: usize, total: usize) {
        self.playing = false;
        self.frame = frame.min(total);
    }

    /// At (or past) the end, Play restarts from 0.
    pub fn play(&mut self, total: usize, now: f64) {
        if total == 0 {
            return;
        }
        if self.frame >= total {
            self.frame = 0;
        }
        self.playing = true;
        self.next_tick = now + self.interval_ms / 1000.0;
    }

    pub fn toggle(&mut self, total: usize, now: f64) {
        if self.playing {
            self.playing = false
        } else {
            self.play(total, now)
        }
    }

    /// Advances while playing; stops (no loop) at the last frame. Returns the
    /// seconds until the next tick when still playing.
    pub fn tick(&mut self, total: usize, now: f64) -> Option<f64> {
        self.frame = self.frame.min(total);
        if !self.playing {
            return None;
        }
        while now >= self.next_tick {
            if self.frame >= total {
                self.playing = false;
                return None;
            }
            self.frame += 1;
            self.next_tick += self.interval_ms.max(1.0) / 1000.0;
        }
        Some(self.next_tick - now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::ThreadPath;

    fn layers() -> Vec<ThreadLayer> {
        let mut l = ThreadLayer::new("L");
        let ids = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        l.thread_paths.push(ThreadPath::new(
            ids(&["a", "b", "c"]),
            vec!["#f00".into()],
            1.0,
            6.0,
        ));
        l.thread_paths.push(ThreadPath::new(
            ids(&["c", "d"]),
            vec!["#f00".into()],
            1.0,
            6.0,
        ));
        vec![l]
    }

    #[test]
    fn frames_and_truncation() {
        let l = layers();
        assert_eq!(total_thread_frames(&l), 3);
        let t = truncate_at_frame(&l, 1);
        assert_eq!(t[0].thread_paths.len(), 1);
        assert_eq!(t[0].thread_paths[0].pin_ids, vec!["a", "b"]);
        assert!(truncate_at_frame(&l, 0)[0].thread_paths.is_empty());
        assert_eq!(truncate_at_frame(&l, 9)[0].thread_paths.len(), 2);
    }

    #[test]
    fn transport_plays_to_end_and_restarts() {
        let mut t = Transport {
            interval_ms: 100.0,
            ..Default::default()
        };
        t.play(3, 0.0);
        t.tick(3, 0.25);
        assert_eq!(t.frame, 2);
        t.tick(3, 1.0);
        assert_eq!(t.frame, 3);
        assert!(!t.playing);
        t.play(3, 1.0);
        assert_eq!(t.frame, 0);
        t.go_to(99, 3);
        assert_eq!((t.frame, t.playing), (3, false));
    }
}
