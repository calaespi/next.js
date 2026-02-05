//! Boundary type constants for Next.js module boundaries.
//!
//! These are used with the `boundary` field on `ProcessResult::Module` and
//! `ModuleResolveResultItem::Module` to mark modules that cross framework
//! boundaries (e.g., server components, client references).

use once_cell::sync::Lazy;
use turbo_rcstr::{RcStr, rcstr};

/// Server component boundary - marks modules that are React Server Components.
pub fn boundary_type_server_component() -> RcStr {
    rcstr!("server-component")
}

/// Server utility boundary - marks modules decorated with 'use server'.
pub fn boundary_type_server_utility() -> RcStr {
    rcstr!("server-utility")
}

/// Dynamic import entry - marks modules loaded via next/dynamic.
pub fn boundary_type_dynamic_entry() -> RcStr {
    rcstr!("dynamic-entry")
}

/// Ecmascript client reference - marks client components referenced from server components.
pub fn boundary_type_client_reference() -> RcStr {
    rcstr!("client-reference")
}

/// CSS client reference - marks CSS modules referenced from server components.
pub fn boundary_type_css_client_reference() -> RcStr {
    rcstr!("css-client-reference")
}

/// Merge tag for server utility chunk groups.
pub static SERVER_UTILITY_MERGE_TAG: Lazy<RcStr> = Lazy::new(|| rcstr!("next-server-utility"));
