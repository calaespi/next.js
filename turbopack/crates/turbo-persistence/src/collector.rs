use std::mem::take;

use crate::{
    FamilyKind, ValueBuffer,
    collector_entry::{CollectorEntry, CollectorEntryValue, EntryKey, TINY_VALUE_THRESHOLD},
    constants::{
        DATA_THRESHOLD_PER_INITIAL_FILE, MAX_ENTRIES_PER_INITIAL_FILE, MAX_SMALL_VALUE_SIZE,
    },
    key::{StoreKey, hash_key},
};

/// A collector accumulates entries that should be eventually written to a file. It keeps track of
/// count and size of the entries to decide when it's "full". Accessing the entries sorts them.
pub struct Collector<K: StoreKey, const SIZE_SHIFT: usize = 0> {
    total_key_size: usize,
    total_value_size: usize,
    entries: Vec<CollectorEntry<K>>,
}

impl<K: StoreKey, const SIZE_SHIFT: usize> Collector<K, SIZE_SHIFT> {
    /// Creates a new collector. Note that this allocates the full capacity for the entries.
    pub fn new() -> Self {
        Self {
            total_key_size: 0,
            total_value_size: 0,
            entries: Vec::with_capacity(MAX_ENTRIES_PER_INITIAL_FILE >> SIZE_SHIFT),
        }
    }

    /// Returns true if the collector has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns true if the collector is full.
    pub fn is_full(&self) -> bool {
        self.entries.len() >= MAX_ENTRIES_PER_INITIAL_FILE >> SIZE_SHIFT
            || self.total_key_size + self.total_value_size
                > DATA_THRESHOLD_PER_INITIAL_FILE >> SIZE_SHIFT
    }

    /// Adds a normal key-value pair to the collector.
    pub fn put(&mut self, key: K, value: ValueBuffer) {
        let key = EntryKey {
            hash: hash_key(&key),
            data: key,
        };
        let value = if value.len() > MAX_SMALL_VALUE_SIZE {
            CollectorEntryValue::Medium {
                value: value.into_boxed_slice(),
            }
        } else if value.len() <= TINY_VALUE_THRESHOLD {
            let slice: &[u8] = &value;
            let mut arr = [0u8; TINY_VALUE_THRESHOLD];
            arr[..slice.len()].copy_from_slice(slice);
            CollectorEntryValue::Tiny {
                value: arr,
                len: slice.len() as u8,
            }
        } else {
            CollectorEntryValue::Small {
                value: value.into_boxed_slice(),
            }
        };
        self.total_key_size += key.len();
        self.total_value_size += value.len();
        self.entries.push(CollectorEntry { key, value });
    }

    /// Adds a blob key-value pair to the collector.
    pub fn put_blob(&mut self, key: K, blob: u32) {
        let key = EntryKey {
            hash: hash_key(&key),
            data: key,
        };
        self.total_key_size += key.len();
        self.entries.push(CollectorEntry {
            key,
            value: CollectorEntryValue::Large { blob },
        });
    }

    /// Adds a tombstone pair to the collector.
    pub fn delete(&mut self, key: K) {
        let key = EntryKey {
            hash: hash_key(&key),
            data: key,
        };
        self.total_key_size += key.len();
        self.entries.push(CollectorEntry {
            key,
            value: CollectorEntryValue::Deleted,
        });
    }

    /// Adds an entry from another collector to this collector.
    pub fn add_entry(&mut self, entry: CollectorEntry<K>) {
        self.total_key_size += entry.key.len();
        self.total_value_size += entry.value.len();
        self.entries.push(entry);
    }

    /// Sorts and deduplicates entries according to the family kind, returning the entries
    /// in (key, value) order suitable for SST storage.
    ///
    /// For `SingleValue`: only the last entry per key is kept (latest write wins).
    /// For `MultiValue`: deletes discard all prior entries for that key within this batch,
    /// but the tombstone itself is kept to shadow older SSTs. Duplicate values are also removed.
    pub fn sorted(&mut self, kind: FamilyKind) -> (&[CollectorEntry<K>], usize) {
        match kind {
            FamilyKind::SingleValue => {
                // Stable sort by key — preserves insertion order for entries with the same key
                self.entries.sort_by(|a, b| a.key.cmp(&b.key));
                // Keep only the last entry per key (latest write wins).
                // After this, there's exactly one entry per key so the order is already
                // fully determined by key alone — no need to re-sort by (key, value).
                self.entries.dedup_by(|a, b| {
                    if a.key == b.key {
                        std::mem::swap(a, b);
                        true
                    } else {
                        false
                    }
                });
            }
            FamilyKind::MultiValue => {
                // Stable sort by key — preserves insertion order so we can find the
                // last Deleted tombstone per key group.
                self.entries.sort_by(|a, b| a.key.cmp(&b.key));
                // If any Deleted tombstones exist, prune entries before the last
                // tombstone within each key group. Tombstones are rare.
                self.prune_deleted_groups();
                // Re-sort by (key, value) for SST storage and dedup identical pairs.
                // Data is already sorted by key, so only values within key groups need
                // reordering. Using stable sort (timsort) which is O(n) on nearly-sorted
                // data since it detects and merges existing sorted runs.
                self.entries.sort();
                self.entries.dedup();
            }
        }

        self.recalculate_sizes();
        (&self.entries, self.total_key_size)
    }

    /// For MultiValue families: within each key group (stably sorted by key, preserving
    /// insertion order), if a Deleted tombstone is present, discard all entries before the
    /// last tombstone. The tombstone itself plus any entries after it survive.
    ///
    /// Tombstones are rare, so we scan for them directly rather than iterating every group.
    fn prune_deleted_groups(&mut self) {
        let mut i = self.entries.len();
        while i > 0 {
            // Find the last Deleted entry in entries[..i]
            let Some(del_pos) = self.entries[..i]
                .iter()
                .rposition(|e| matches!(e.value, CollectorEntryValue::Deleted))
            else {
                break;
            };

            // Scan backwards to find the start of this key group
            let key = &self.entries[del_pos].key;
            let mut group_start = del_pos;
            while group_start > 0 && self.entries[group_start - 1].key == *key {
                group_start -= 1;
            }

            // Remove entries before the tombstone (group_start..del_pos)
            self.entries.drain(group_start..del_pos);
            i = group_start;
        }
    }

    /// Recalculates total_key_size and total_value_size from entries.
    fn recalculate_sizes(&mut self) {
        self.total_key_size = 0;
        self.total_value_size = 0;
        for entry in &self.entries {
            self.total_key_size += entry.key.len();
            self.total_value_size += entry.value.len();
        }
    }

    /// Clears the collector.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_key_size = 0;
        self.total_value_size = 0;
    }

    /// Drains all entries from the collector in un-sorted order. This can be used to move the
    /// entries into another collector.
    pub fn drain(&mut self) -> impl Iterator<Item = CollectorEntry<K>> + '_ {
        self.total_key_size = 0;
        self.total_value_size = 0;
        self.entries.drain(..)
    }

    /// Clears the collector and drops the capacity
    pub fn drop_contents(&mut self) {
        drop(take(&mut self.entries));
        self.total_key_size = 0;
        self.total_value_size = 0;
    }
}
