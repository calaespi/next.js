# Module Boundary Refactoring Plan

## Goal

Eliminate wrapper module types (NextServerComponentModule, NextServerUtilityModule, NextDynamicEntryModule) by moving boundary detection from type-based to boundary info-based.

## Completed ✅

### Phase 1-4: Core Infrastructure (turbopack-core)

- Created `boundary.rs` with `BoundaryInfo` struct (RcStr boundary_type + optional source_path)
- Extended `ProcessResult` with `ModuleWithBoundary` variant
- Extended `ModuleResolveResultItem` with `ModuleWithBoundary` variant
- Created `ResolvedModule` struct in `reference/mod.rs` to carry boundary info through resolution

### Phase 5: Boundary Type Constants (next-core)

- Created `boundary_types.rs` with RcStr constants for each boundary type
- Keeps turbopack generic - specific boundary types defined in next-core

### Phase 6: Transitions Updated

- `NextServerComponentTransition` - returns `ProcessResult::ModuleWithBoundary`
- `NextServerUtilityTransition` - returns `ProcessResult::ModuleWithBoundary`
- `NextDynamicTransition` - returns `ProcessResult::ModuleWithBoundary`

### Phase 7: Type System Updated

- Changed `ClientReference.server_component` from `Option<ResolvedVc<NextServerComponentModule>>` to `Option<ServerComponentEntry>`
- Created `ServerComponentEntry` struct with `module` and optional `boundary` info
- Created `ServerUtilEntry` struct with `module`
- Updated `ClientReferenceGraphResult` to use new entry types
- Updated all consumers:
  - `app.rs` - fixed module access patterns
  - `app_client_references_chunks.rs` - updated map keys and module access
  - `client_reference_manifest.rs` - updated source path lookup
  - `module_graph.rs` - transitional code creates entries from type detection

### Phase 8: Module Graph Extended

- Extended `SingleModuleGraphNode::Module` to include optional boundary info
- Extended `SingleModuleGraphBuilderNode::Module` to include optional boundary info
- Added `iter_nodes_with_boundary()` method to `SingleModuleGraph` and `ModuleGraphSnapshot`
- Updated graph building to preserve boundary info from `ResolvedModule`

### Phase 9: map_client_references Updated

- Updated `map_client_references` in `client_references.rs` to check boundary info first
- Added `ServerComponentBoundary` and `ServerUtilityBoundary` variants to `ClientManifestEntryType`
- Removed type-based detection fallback for server components/utilities

### Phase 10: Wrapper Modules Deleted

- Deleted `NextServerComponentModule` and `NextServerComponentModuleReference`
- Deleted `NextServerUtilityModule` and `NextServerUtilityModuleReference`
- Deleted `NextDynamicEntryModule`
- Moved `NEXT_SERVER_UTILITY_MERGE_TAG` to `boundary_types.rs` as `SERVER_UTILITY_MERGE_TAG`
- Updated `map_next_dynamic` in `dynamic_imports.rs` to use boundary info
- Updated `DynamicImportedChunks` and `DynamicImportEntriesWithImporter` to use `ResolvedVc<Box<dyn Module>>`
- Removed all transitional type-based detection fallbacks

### Phase 11: Unified ProcessResult and ModuleResolveResultItem Variants

- Merged `ProcessResult::Module` and `ProcessResult::ModuleWithBoundary` into single `Module` variant:
  ```rust
  ProcessResult::Module {
      module: ResolvedVc<Box<dyn Module>>,
      boundary: Option<ResolvedVc<BoundaryInfo>>,
  }
  ```
- Merged `ModuleResolveResultItem::Module` and `ModuleResolveResultItem::ModuleWithBoundary` similarly
- Updated all transitions to use the unified pattern with `boundary: Some(...)` or `boundary: None`
- Updated all pattern matches throughout turbopack and next crates

## Current State

**Goal achieved!** The wrapper module types have been eliminated. Boundary detection is now fully based on `BoundaryInfo`, and the enum variants have been unified.

Key changes:

1. **Unified enum variants**: `ProcessResult::Module` and `ModuleResolveResultItem::Module` now have an optional `boundary` field instead of separate `ModuleWithBoundary` variants. This simplifies pattern matching and reduces code duplication.

2. **Boundary info flows through resolution**: When a module is processed through a boundary transition (server component, server utility, dynamic), the boundary info is attached to `ProcessResult::Module { boundary: Some(...) }` and propagates through resolution.

3. **Boundary info preserved in module graph**: `SingleModuleGraphNode` now carries optional boundary info, which is populated from `ResolvedModule` during graph building.

4. **New entry types**: `ServerComponentEntry` and `ServerUtilEntry` carry both the module reference and optional boundary info. This allows consumers to access source paths and other metadata.

5. **Boundary-based detection**: All detection (`find_server_entries`, `map_client_references`, `map_next_dynamic`) now uses boundary info exclusively (except for client reference modules which still need their wrapper types to hold client/SSR module refs).

## Key Files Changed

### Turbopack Core (new infrastructure)

- `turbopack/crates/turbopack-core/src/boundary.rs` - BoundaryInfo definition
- `turbopack/crates/turbopack-core/src/context.rs` - ProcessResult::Module with optional boundary
- `turbopack/crates/turbopack-core/src/resolve/mod.rs` - ModuleResolveResultItem::Module with optional boundary
- `turbopack/crates/turbopack-core/src/reference/mod.rs` - ResolvedModule struct
- `turbopack/crates/turbopack-core/src/module_graph/mod.rs` - SingleModuleGraphNode with boundary, iter_nodes_with_boundary

### Next Core (transitions and detection)

- `crates/next-core/src/boundary_types.rs` - Boundary type constants + SERVER_UTILITY_MERGE_TAG
- `crates/next-core/src/next_server_component/server_component_transition.rs` - Returns boundary info
- `crates/next-core/src/next_server_utility/server_utility_transition.rs` - Returns boundary info
- `crates/next-core/src/next_dynamic/dynamic_transition.rs` - Returns boundary info
- `crates/next-core/src/next_client_reference/visit_client_reference.rs` - Entry types and detection
- `crates/next-core/src/next_app/app_client_references_chunks.rs`
- `crates/next-core/src/next_manifests/client_reference_manifest.rs`

### Next API (consumers)

- `crates/next-api/src/app.rs` - Updated imports
- `crates/next-api/src/module_graph.rs` - Updated detection
- `crates/next-api/src/client_references.rs` - Boundary-based detection
- `crates/next-api/src/dynamic_imports.rs` - Boundary-based detection

### Deleted Files

- `crates/next-core/src/next_server_component/server_component_module.rs`
- `crates/next-core/src/next_server_component/server_component_reference.rs`
- `crates/next-core/src/next_server_utility/server_utility_module.rs`
- `crates/next-core/src/next_server_utility/server_utility_reference.rs`
- `crates/next-core/src/next_dynamic/dynamic_module.rs`
