//! Module boundary types for tracking boundary crossings in the module graph.
//!
//! Boundaries represent transitions between different module contexts (e.g., server components,
//! client references). Instead of using wrapper modules to mark these boundaries, the boundary
//! information is attached to the resolution result directly via an optional `boundary` field
//! in `ProcessResult::Module` and `ModuleResolveResultItem::Module`.
//!
//! The boundary type is an RcStr to keep turbopack generic - specific boundary type constants
//! should be defined in the framework-specific crates (e.g., next-core).

use turbo_rcstr::RcStr;
use turbo_tasks_fs::FileSystemPath;

/// Metadata associated with a boundary crossing.
///
/// This is stored in the optional `boundary` field of `ProcessResult::Module` and
/// `ModuleResolveResultItem::Module` to indicate that a module crossing represents
/// a boundary without wrapping the module in a marker type.
#[turbo_tasks::value(shared)]
#[derive(Clone, Debug)]
pub struct BoundaryInfo {
    /// The type of boundary being crossed (e.g., "server-component", "client-reference").
    /// This is an RcStr to keep turbopack generic - specific values are defined in
    /// framework-specific crates.
    pub boundary_type: RcStr,
    /// Original source path before any transformations (e.g., before MDX -> MDX.tsx).
    /// Used for server components where the manifest key needs the original path.
    pub source_path: Option<FileSystemPath>,
}

impl BoundaryInfo {
    /// Create a new boundary info with the given type and no source path
    pub fn new(boundary_type: RcStr) -> Self {
        Self {
            boundary_type,
            source_path: None,
        }
    }

    /// Create a new boundary info with the given type and source path
    pub fn with_source_path(boundary_type: RcStr, source_path: FileSystemPath) -> Self {
        Self {
            boundary_type,
            source_path: Some(source_path),
        }
    }
}
