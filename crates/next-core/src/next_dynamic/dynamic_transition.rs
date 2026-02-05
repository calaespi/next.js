use anyhow::{Result, bail};
use turbo_tasks::{ResolvedVc, Vc};
use turbopack::{ModuleAssetContext, transition::Transition};
use turbopack_core::{
    boundary::BoundaryInfo,
    context::{AssetContext, ProcessResult},
    reference_type::ReferenceType,
    source::Source,
};
use turbopack_ecmascript::chunk::EcmascriptChunkPlaceable;

use crate::boundary_types::boundary_type_dynamic_entry;

/// This transition is used to mark a module as a dynamic import entry.
/// Optionally, it can also apply another transition (i.e. to the client context).
///
/// Instead of wrapping the module in a marker type, this transition attaches
/// boundary metadata to the ProcessResult, which is then propagated through
/// the module resolution infrastructure.
///
/// This will get picked up during module processing and will be used to
/// create the dynamic entry, and the dynamic manifest entry.
#[turbo_tasks::value]
pub struct NextDynamicTransition {
    client_transition: Option<ResolvedVc<Box<dyn Transition>>>,
}

#[turbo_tasks::value_impl]
impl NextDynamicTransition {
    /// Create a transition that only marks the module with a dynamic entry boundary.
    #[turbo_tasks::function]
    pub fn new_marker() -> Vc<Self> {
        NextDynamicTransition {
            client_transition: None,
        }
        .cell()
    }

    /// Create a transition that applies `client_transition` and marks the module
    /// with a dynamic entry boundary.
    #[turbo_tasks::function]
    pub fn new_client(client_transition: ResolvedVc<Box<dyn Transition>>) -> Vc<Self> {
        NextDynamicTransition {
            client_transition: Some(client_transition),
        }
        .cell()
    }
}

#[turbo_tasks::value_impl]
impl Transition for NextDynamicTransition {
    #[turbo_tasks::function]
    async fn process(
        self: Vc<Self>,
        source: Vc<Box<dyn Source>>,
        module_asset_context: Vc<ModuleAssetContext>,
        _reference_type: ReferenceType,
    ) -> Result<Vc<ProcessResult>> {
        let module_asset_context = self.process_context(module_asset_context);
        let process_result = match self.await?.client_transition {
            Some(client_transition) => {
                client_transition
                    .process(source, module_asset_context, ReferenceType::Undefined)
                    .await?
            }
            None => {
                module_asset_context
                    .process(source, ReferenceType::Undefined)
                    .await?
            }
        };

        Ok(match &*process_result {
            ProcessResult::Module { module, .. } => {
                let Some(client_module) =
                    ResolvedVc::try_sidecast::<Box<dyn EcmascriptChunkPlaceable>>(*module)
                else {
                    bail!("not an ecmascript module");
                };

                // Return the module with boundary info attached
                // (override any existing boundary with DynamicEntry)
                ProcessResult::Module {
                    module: ResolvedVc::upcast(client_module),
                    boundary: Some(
                        BoundaryInfo::new(boundary_type_dynamic_entry()).resolved_cell(),
                    ),
                }
                .cell()
            }
            ProcessResult::Unknown(_) | ProcessResult::Ignore => ProcessResult::Ignore.cell(),
        })
    }
}
