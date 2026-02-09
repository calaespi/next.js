use std::cmp::Ordering;

use crate::{
    constants::MAX_INLINE_VALUE_SIZE,
    key::StoreKey,
    static_sorted_file_builder::{Entry, EntryValue},
};

/// Classification of a value for sorting purposes.
///
/// Both `CollectorEntryValue` (write path) and `LazyLookupValue` (read/merge path) use
/// this same classification to ensure a consistent total order:
/// `Deleted < ByteContent(ordered by bytes) < Blob(ordered by sequence number)`.
pub enum ValueCategory<'a> {
    Deleted,
    ByteContent(&'a [u8]),
    Blob(u32),
}

/// Compare two values using the canonical sort order: Deleted < ByteContent < Blob.
///
/// This is the single source of truth for value ordering, used by both
/// `CollectorEntryValue::Ord` and `LazyLookupValue::cmp_for_dedup`.
pub fn cmp_value_category(a: &ValueCategory<'_>, b: &ValueCategory<'_>) -> Ordering {
    fn type_order(v: &ValueCategory<'_>) -> u8 {
        match v {
            ValueCategory::Deleted => 0,
            ValueCategory::ByteContent(_) => 1,
            ValueCategory::Blob(_) => 2,
        }
    }
    match type_order(a).cmp(&type_order(b)) {
        Ordering::Equal => match (a, b) {
            (ValueCategory::Deleted, ValueCategory::Deleted) => Ordering::Equal,
            (ValueCategory::ByteContent(a), ValueCategory::ByteContent(b)) => a.cmp(b),
            (ValueCategory::Blob(a), ValueCategory::Blob(b)) => a.cmp(b),
            _ => unreachable!(),
        },
        ord => ord,
    }
}

pub struct CollectorEntry<K: StoreKey> {
    pub key: EntryKey<K>,
    pub value: CollectorEntryValue,
}

impl<K: StoreKey> PartialEq for CollectorEntry<K> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.value == other.value
    }
}

impl<K: StoreKey> Eq for CollectorEntry<K> {}

impl<K: StoreKey> PartialOrd for CollectorEntry<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: StoreKey> Ord for CollectorEntry<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key
            .cmp(&other.key)
            .then_with(|| self.value.cmp(&other.value))
    }
}

/// The size threshold for inline storage in CollectorEntryValue, this is the largest value that can
/// be stored inline without inflating the size of the enum
pub const TINY_VALUE_THRESHOLD: usize = 22;

pub enum CollectorEntryValue {
    /// Tiny value stored inline (22 16 bytes, no heap allocation)
    Tiny {
        value: [u8; TINY_VALUE_THRESHOLD],
        len: u8,
    },
    /// Small value that fits in shared value blocks (> 16 bytes, ≤ MAX_SMALL_VALUE_SIZE)
    Small {
        value: Box<[u8]>,
    },
    /// Medium value that gets its own value block (> MAX_SMALL_VALUE_SIZE)
    Medium {
        value: Box<[u8]>,
    },
    Large {
        blob: u32,
    },
    Deleted,
}

impl CollectorEntryValue {
    pub fn len(&self) -> usize {
        match self {
            CollectorEntryValue::Tiny { len, .. } => *len as usize,
            CollectorEntryValue::Small { value } => value.len(),
            CollectorEntryValue::Medium { value } => value.len(),
            CollectorEntryValue::Large { blob: _ } => 0,
            CollectorEntryValue::Deleted => 0,
        }
    }

    /// Returns the byte slice for byte-content values, or None for Large/Deleted.
    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            CollectorEntryValue::Tiny { value, len } => Some(&value[..*len as usize]),
            CollectorEntryValue::Small { value } => Some(value),
            CollectorEntryValue::Medium { value } => Some(value),
            CollectorEntryValue::Large { .. } | CollectorEntryValue::Deleted => None,
        }
    }

    /// Classify this value for sort ordering purposes.
    pub fn category(&self) -> ValueCategory<'_> {
        match self {
            CollectorEntryValue::Deleted => ValueCategory::Deleted,
            CollectorEntryValue::Large { blob } => ValueCategory::Blob(*blob),
            _ => ValueCategory::ByteContent(self.as_bytes().unwrap()),
        }
    }
}

impl PartialEq for CollectorEntryValue {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for CollectorEntryValue {}

impl PartialOrd for CollectorEntryValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CollectorEntryValue {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_value_category(&self.category(), &other.category())
    }
}

pub struct EntryKey<K: StoreKey> {
    pub hash: u64,
    pub data: K,
}

impl<K: StoreKey> EntryKey<K> {
    pub fn len(&self) -> usize {
        std::mem::size_of::<u64>() + self.data.len()
    }
}

impl<K: StoreKey> PartialEq for EntryKey<K> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.data == other.data
    }
}

impl<K: StoreKey> Eq for EntryKey<K> {}

impl<K: StoreKey> PartialOrd for EntryKey<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: StoreKey> Ord for EntryKey<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hash
            .cmp(&other.hash)
            .then_with(|| self.data.cmp(&other.data))
    }
}

impl<K: StoreKey> Entry for CollectorEntry<K> {
    fn key_hash(&self) -> u64 {
        self.key.hash
    }

    fn key_len(&self) -> usize {
        self.key.data.len()
    }

    fn write_key_to(&self, buf: &mut Vec<u8>) {
        self.key.data.write_to(buf);
    }

    fn value(&self) -> EntryValue<'_> {
        match &self.value {
            CollectorEntryValue::Tiny { value, len } => {
                let slice = &value[..*len as usize];
                if slice.len() <= MAX_INLINE_VALUE_SIZE {
                    EntryValue::Inline { value: slice }
                } else {
                    EntryValue::Small { value: slice }
                }
            }
            CollectorEntryValue::Small { value } => {
                if value.len() <= MAX_INLINE_VALUE_SIZE {
                    EntryValue::Inline { value }
                } else {
                    EntryValue::Small { value }
                }
            }
            CollectorEntryValue::Medium { value } => EntryValue::Medium { value },
            CollectorEntryValue::Large { blob } => EntryValue::Large { blob: *blob },
            CollectorEntryValue::Deleted => EntryValue::Deleted,
        }
    }
}
