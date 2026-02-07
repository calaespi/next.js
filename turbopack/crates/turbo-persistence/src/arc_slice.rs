use std::{
    borrow::Borrow,
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    io::{self, Read},
    ops::{Deref, Range},
    sync::Arc,
};

/// A byte slice that is either borrowed or backed by an `Arc<[u8]>`.
///
/// - `Borrowed`: zero-copy reference into existing memory (e.g. an mmap'd file)
/// - `Owned`: reference-counted heap allocation (e.g. decompressed block data)
///
/// The owned variant can be sub-sliced: the `data` pointer may point to a
/// sub-range of the `Arc<[u8]>`, while the Arc keeps the full allocation alive.
#[derive(Clone)]
pub enum ArcSlice<'a> {
    Borrowed(&'a [u8]),
    Owned {
        data: *const [u8],
        #[allow(dead_code)]
        arc: Arc<[u8]>,
    },
}

unsafe impl Send for ArcSlice<'_> {}
unsafe impl Sync for ArcSlice<'_> {}

impl<'a> From<Arc<[u8]>> for ArcSlice<'a> {
    fn from(arc: Arc<[u8]>) -> Self {
        Self::Owned {
            data: &*arc as *const [u8],
            arc,
        }
    }
}

impl<'a> From<Box<[u8]>> for ArcSlice<'a> {
    fn from(b: Box<[u8]>) -> Self {
        Self::from(Arc::from(b))
    }
}

impl<'a> From<&'a [u8]> for ArcSlice<'a> {
    fn from(slice: &'a [u8]) -> Self {
        Self::Borrowed(slice)
    }
}

impl Deref for ArcSlice<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            ArcSlice::Borrowed(slice) => slice,
            ArcSlice::Owned { data, .. } => unsafe { &**data },
        }
    }
}

impl Borrow<[u8]> for ArcSlice<'_> {
    fn borrow(&self) -> &[u8] {
        self
    }
}

impl Hash for ArcSlice<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl PartialEq for ArcSlice<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl Debug for ArcSlice<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl Eq for ArcSlice<'_> {}

impl Read for ArcSlice<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let available = &**self;
        let len = std::cmp::min(buf.len(), available.len());
        buf[..len].copy_from_slice(&available[..len]);
        // Advance the slice view
        match self {
            ArcSlice::Borrowed(slice) => *slice = &slice[len..],
            ArcSlice::Owned { data, .. } => {
                let full = unsafe { &**data };
                *data = &full[len..] as *const [u8];
            }
        }
        Ok(len)
    }
}

impl<'a> ArcSlice<'a> {
    /// Returns a new `ArcSlice` that points to a sub-range of the current slice.
    pub fn slice(self, range: Range<usize>) -> ArcSlice<'a> {
        match self {
            ArcSlice::Borrowed(slice) => ArcSlice::Borrowed(&slice[range]),
            ArcSlice::Owned { data, arc } => {
                let full = unsafe { &*data };
                ArcSlice::Owned {
                    data: &full[range] as *const [u8],
                    arc,
                }
            }
        }
    }

    /// Creates a sub-slice from a slice reference that points into this ArcSlice's backing data.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `subslice` points to memory within this ArcSlice's
    /// backing storage (not just within the current slice view, but anywhere in the original
    /// backing).
    pub unsafe fn slice_from_subslice(&self, subslice: &'a [u8]) -> ArcSlice<'a> {
        match self {
            ArcSlice::Borrowed(_) => ArcSlice::Borrowed(subslice),
            ArcSlice::Owned { arc, .. } => ArcSlice::Owned {
                data: subslice as *const [u8],
                arc: arc.clone(),
            },
        }
    }

    /// Converts to an owned `ArcSlice<'static>` by ensuring the data is in an Arc.
    /// For borrowed slices, this copies the data into a new Arc allocation.
    /// For owned slices, this is free (just moves the Arc).
    pub fn into_owned(self) -> ArcSlice<'static> {
        match self {
            ArcSlice::Borrowed(slice) => ArcSlice::from(Arc::from(slice)),
            ArcSlice::Owned { data, arc } => ArcSlice::Owned { data, arc },
        }
    }
}
