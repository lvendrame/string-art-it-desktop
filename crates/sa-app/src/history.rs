//! Linear undo/redo (spec 10). Port of web `HistoryStack` + `SetValueCommand`:
//! every undoable action records a before/after snapshot of the affected state,
//! so cascading effects (pin erase + thread splits) undo as one step.

/// Snapshot history over `T`. Running a new entry clears the redo stack.
#[derive(Clone, Debug)]
pub struct History<T> {
    undo: Vec<Entry<T>>,
    redo: Vec<Entry<T>>,
    limit: usize,
}

#[derive(Clone, Debug)]
struct Entry<T> {
    before: T,
    after: T,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: 500,
        }
    }
}

impl<T: Clone> History<T> {
    /// Records a completed change. Caller has already applied `after`.
    pub fn record(&mut self, before: T, after: T) {
        self.undo.push(Entry { before, after });
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Replaces the `after` of the newest entry (coalesced continuous edits).
    pub fn amend_last(&mut self, after: T) -> bool {
        match self.undo.last_mut() {
            Some(entry) => {
                entry.after = after;
                self.redo.clear();
                true
            }
            None => false,
        }
    }

    /// Returns the state to restore, if any.
    pub fn undo(&mut self) -> Option<T> {
        let entry = self.undo.pop()?;
        let before = entry.before.clone();
        self.redo.push(entry);
        Some(before)
    }

    pub fn redo(&mut self) -> Option<T> {
        let entry = self.redo.pop()?;
        let after = entry.after.clone();
        self.undo.push(entry);
        Some(after)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Loading a document starts a fresh history (spec 16).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Number of undoable entries (used for dirty tracking).
    pub fn depth(&self) -> usize {
        self.undo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_and_redo_invalidation() {
        let mut h = History::default();
        h.record(0, 1);
        assert_eq!(h.undo(), Some(0));
        assert_eq!(h.redo(), Some(1));
        assert_eq!(h.undo(), Some(0));
        h.record(0, 1); // new change after undo clears redo
        assert_eq!(h.redo(), None);
        assert!(!h.can_redo());
        h.clear();
        assert!(!h.can_undo());
        assert_eq!(h.undo(), None);
    }
}
