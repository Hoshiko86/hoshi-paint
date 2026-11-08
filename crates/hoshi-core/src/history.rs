//! Unlimited session timeline. Archived states hold disk references, not RAM pixels.
use crate::{
    Error, State,
    history_store::{HistoryStore, StateRecord},
};
use std::path::{Path, PathBuf};
struct Entry {
    record: StateRecord,
    label: &'static str,
    selected: usize,
}
pub(crate) struct History {
    store: Option<HistoryStore>,
    entries: Vec<Entry>,
    cursor: usize,
    generation: u64,
    structure_generation: u64,
    directory: Option<PathBuf>,
    compact_pending: bool,
}
impl Default for History {
    fn default() -> Self {
        Self {
            store: None,
            entries: Vec::new(),
            cursor: 0,
            generation: 0,
            structure_generation: 0,
            directory: std::env::var_os("HOSHI_HISTORY_DIR").map(PathBuf::from),
            compact_pending: false,
        }
    }
}
impl History {
    #[cfg(test)]
    pub(crate) fn store_mut(&mut self) -> Option<&mut HistoryStore> {
        self.store.as_mut()
    }
    pub fn set_directory(&mut self, path: &Path) -> Result<(), Error> {
        if !self.entries.is_empty() {
            return Err(Error::InvalidHistoryStep);
        }
        self.store = None;
        self.directory = Some(path.to_path_buf());
        Ok(())
    }
    pub fn position(&self) -> usize {
        self.cursor
    }
    pub fn len(&self) -> usize {
        self.entries.len().max(1)
    }
    pub fn redo_len(&self) -> usize {
        self.len() - self.cursor - 1
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn structure_generation(&self) -> u64 {
        self.structure_generation
    }
    pub fn cleanup_pending(&self) -> bool {
        self.compact_pending
    }
    pub fn bytes(&self) -> u64 {
        self.store.as_ref().map_or(0, HistoryStore::bytes)
    }
    pub fn label(&self, index: usize) -> Option<&'static str> {
        if index == 0 {
            Some("Oldest available state")
        } else {
            self.entries.get(index).map(|e| e.label)
        }
    }
    pub fn checkpoint(
        &mut self,
        before: &State,
        after: &State,
        label: &'static str,
    ) -> Result<(), Error> {
        // Reserve timeline metadata before touching disk or the redo branch.
        let desired = self.cursor.checked_add(2).ok_or(Error::MemoryLimit)?;
        if desired > self.entries.len() {
            self.entries
                .try_reserve(desired - self.entries.len())
                .map_err(|_| Error::MemoryLimit)?;
        }
        if self.store.is_none() {
            self.store = Some(HistoryStore::new(self.directory.as_deref())?)
        }
        let capture_before = self.entries.is_empty();
        let records = if capture_before {
            self.store
                .as_mut()
                .expect("created archive")
                .archive_many(&[before, after])?
        } else {
            self.store
                .as_mut()
                .expect("created archive")
                .archive_many(&[after])?
        };
        let after_record = *records.last().expect("one archived state");
        let branched = self.redo_len() > 0;
        if self.entries.is_empty() {
            self.entries.push(Entry {
                record: records[0],
                label: "Oldest available state",
                selected: before.selected,
            })
        } else {
            // Layer selection is navigation metadata, not an image edit.
            self.entries[self.cursor].selected = before.selected;
        }
        if branched {
            self.structure_generation = self.structure_generation.wrapping_add(1);
        }
        self.entries.truncate(self.cursor + 1);
        self.entries.push(Entry {
            record: after_record,
            label,
            selected: after.selected,
        });
        self.cursor += 1;
        self.generation = self.generation.wrapping_add(1);
        // Disk reclamation is best-effort after the new edit is safely archived.
        // A cleanup failure cannot erase an edit or any reachable history state.
        if branched || self.compact_pending {
            self.compact_pending = !self.compact()
        }
        Ok(())
    }
    pub fn navigate(&mut self, current: &State, target: usize) -> Result<Option<State>, Error> {
        if target >= self.len() {
            return Err(Error::InvalidHistoryStep);
        }
        if target == self.cursor {
            return Ok(None);
        }
        let store = self.store.as_mut().ok_or(Error::InvalidHistoryStep)?;
        // Selection overrides stay in the small session descriptor. Undo/Redo
        // never need disk writes, including after changing the selected layer.
        // Read first so failures leave both descriptors and the cursor intact.
        let mut state = store.restore(self.entries[target].record)?;
        if self.entries[target].selected >= state.layers.len() {
            return Err(Error::HistoryCorrupt);
        }
        state.selected = self.entries[target].selected;
        self.entries[self.cursor].selected = current.selected;
        self.cursor = target;
        self.generation = self.generation.wrapping_add(1);
        Ok(Some(state))
    }
    fn compact(&mut self) -> bool {
        let mut records = Vec::new();
        if records.try_reserve_exact(self.entries.len()).is_err() {
            return false;
        }
        records.extend(self.entries.iter().map(|e| e.record));
        let Some(store) = &mut self.store else {
            return true;
        };
        if store.compact(&mut records).is_err() {
            return false;
        }
        for (entry, record) in self.entries.iter_mut().zip(records) {
            entry.record = record
        }
        true
    }
}
