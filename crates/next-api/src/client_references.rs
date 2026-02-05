use anyhow::Result;
use bincode::{Decode, Encode};
use next_core::{
    boundary_types::{
        boundary_type_client_reference, boundary_type_css_client_reference,
        boundary_type_server_component, boundary_type_server_utility,
    },
    next_client_reference::{CssClientReferenceModule, EcmascriptClientReferenceModule},
};
use rustc_hash::FxHashMap;
use turbo_tasks::{
    NonLocalValue, ResolvedVc, TryFlatJoinIterExt, Vc, debug::ValueDebugFormat, trace::TraceRawVcs,
};
use turbopack_core::{boundary::BoundaryInfo, module::Module, module_graph::ModuleGraphLayer};
use turbopack_css::chunk::CssChunkPlaceable;

#[derive(
    Copy, Clone, Eq, PartialEq, TraceRawVcs, ValueDebugFormat, NonLocalValue, Encode, Decode,
)]
pub enum ClientManifestEntryType {
    EcmascriptClientReference {
        module: ResolvedVc<EcmascriptClientReferenceModule>,
        ssr_module: ResolvedVc<Box<dyn Module>>,
    },
    CssClientReference(ResolvedVc<Box<dyn CssChunkPlaceable>>),
    /// Server component or server utility boundary (boundary_type cached for sync access)
    Boundary {
        boundary: ResolvedVc<BoundaryInfo>,
        is_server_component: bool,
    },
}

/// Tracks information about all the css and js client references in the graph.
#[turbo_tasks::value(transparent)]
pub struct ClientReferenceData(FxHashMap<ResolvedVc<Box<dyn Module>>, ClientManifestEntryType>);

#[turbo_tasks::function]
pub async fn map_client_references(
    graph: ResolvedVc<ModuleGraphLayer>,
) -> Result<Vc<ClientReferenceData>> {
    let graph = graph.await?;
    let manifest = graph
        .iter_nodes_with_boundary()
        .map(|(module, boundary)| async move {
            // All detection is now boundary-based
            let Some(boundary) = boundary else {
                return Ok(None);
            };

            let boundary_info = boundary.await?;
            let boundary_type = &boundary_info.boundary_type;

            if *boundary_type == boundary_type_server_component() {
                return Ok(Some((
                    module,
                    ClientManifestEntryType::Boundary {
                        boundary,
                        is_server_component: true,
                    },
                )));
            }

            if *boundary_type == boundary_type_server_utility() {
                return Ok(Some((
                    module,
                    ClientManifestEntryType::Boundary {
                        boundary,
                        is_server_component: false,
                    },
                )));
            }

            // Client references - downcast to get module-specific data
            if *boundary_type == boundary_type_client_reference() {
                let client_reference_module = ResolvedVc::try_downcast_type::<
                    EcmascriptClientReferenceModule,
                >(module)
                .expect("client reference boundary should be on EcmascriptClientReferenceModule");
                return Ok(Some((
                    module,
                    ClientManifestEntryType::EcmascriptClientReference {
                        module: client_reference_module,
                        ssr_module: ResolvedVc::upcast(client_reference_module.await?.ssr_module),
                    },
                )));
            }

            if *boundary_type == boundary_type_css_client_reference() {
                let client_reference_module = ResolvedVc::try_downcast_type::<
                    CssClientReferenceModule,
                >(module)
                .expect("css client reference boundary should be on CssClientReferenceModule");
                return Ok(Some((
                    module,
                    ClientManifestEntryType::CssClientReference(
                        client_reference_module.await?.client_module,
                    ),
                )));
            }

            Ok(None)
        })
        .try_flat_join()
        .await?
        .into_iter()
        .collect::<FxHashMap<_, _>>();

    Ok(Vc::cell(manifest))
}
