use anyhow::Result;
use turbo_tasks::Vc;
use turbopack::{ModuleAssetContext, transition::Transition};
use turbopack_core::{
    boundary::BoundaryInfo,
    context::{AssetContext, ProcessResult},
    reference_type::ReferenceType,
    source::Source,
};

use crate::boundary_types::boundary_type_server_utility;

/// This transition marks a module as a server utility boundary.
///
/// Instead of wrapping the module in a marker type, this transition attaches
/// boundary metadata to the ProcessResult, which is then propagated through
/// the module resolution infrastructure.
///
/// When walking the module graph to build the client reference manifest, this
/// is used to determine under which server component CSS client references are
/// required. Ultimately, this tells Next.js what CSS to inject into the page.
#[turbo_tasks::value(shared)]
pub struct NextServerUtilityTransition {}

#[turbo_tasks::value_impl]
impl NextServerUtilityTransition {
    /// Creates a new [`Vc<NextServerUtilityTransition>`].
    #[turbo_tasks::function]
    pub fn new() -> Vc<Self> {
        NextServerUtilityTransition {}.cell()
    }
}

#[turbo_tasks::value_impl]
impl Transition for NextServerUtilityTransition {
    #[turbo_tasks::function]
    async fn process(
        self: Vc<Self>,
        source: Vc<Box<dyn Source>>,
        module_asset_context: Vc<ModuleAssetContext>,
        reference_type: ReferenceType,
    ) -> Result<Vc<ProcessResult>> {
        let source = self.process_source(source);
        let module_asset_context = self.process_context(module_asset_context);

        Ok(
            match &*module_asset_context.process(source, reference_type).await? {
                ProcessResult::Module { module, boundary } => {
                    // Return the module with boundary info attached (or preserve existing boundary)
                    ProcessResult::Module {
                        module: *module,
                        boundary: boundary.or(Some(
                            BoundaryInfo::new(boundary_type_server_utility()).resolved_cell(),
                        )),
                    }
                    .cell()
                }
                ProcessResult::Unknown(source) => ProcessResult::Unknown(*source).cell(),
                ProcessResult::Ignore => ProcessResult::Ignore.cell(),
            },
        )
    }
}
