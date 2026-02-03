use anyhow::Result;
use indoc::formatdoc;
use turbo_rcstr::rcstr;
use turbo_tasks::{ResolvedVc, Vc};
use turbo_tasks_fs::FileSystemPath;
use turbopack_core::{
    chunk::{ChunkItem, ChunkType, ChunkableModule, ChunkingContext, ModuleChunkItemIdExt},
    ident::AssetIdent,
    module::{Module, ModuleSideEffects},
    module_graph::ModuleGraph,
    output::OutputAssetsReference,
    reference::{ModuleReference, ModuleReferences},
    source::OptionSource,
};
use turbopack_ecmascript::{
    chunk::{
        EcmascriptChunkItem, EcmascriptChunkItemContent, EcmascriptChunkPlaceable,
        EcmascriptChunkType, EcmascriptExports,
    },
    references::esm::EsmExports,
    runtime_functions::{TURBOPACK_EXPORT_NAMESPACE, TURBOPACK_IMPORT},
    utils::StringifyJs,
};

use super::server_utility_reference::NextServerUtilityModuleReference;

#[turbo_tasks::value(shared)]
pub struct NextServerUtilityModule {
    pub module: ResolvedVc<Box<dyn EcmascriptChunkPlaceable>>,
}

#[turbo_tasks::value_impl]
impl NextServerUtilityModule {
    #[turbo_tasks::function]
    pub fn new(module: ResolvedVc<Box<dyn EcmascriptChunkPlaceable>>) -> Vc<Self> {
        NextServerUtilityModule { module }.cell()
    }

    #[turbo_tasks::function]
    pub fn server_path(&self) -> Vc<FileSystemPath> {
        self.module.ident().path()
    }
}

#[turbo_tasks::value_impl]
impl Module for NextServerUtilityModule {
    #[turbo_tasks::function]
    fn ident(&self) -> Vc<AssetIdent> {
        self.module
            .ident()
            .with_modifier(rcstr!("Next.js server utility"))
    }

    #[turbo_tasks::function]
    fn source(&self) -> Vc<OptionSource> {
        Vc::cell(None)
    }

    #[turbo_tasks::function]
    async fn references(&self) -> Result<Vc<ModuleReferences>> {
        Ok(Vc::cell(vec![ResolvedVc::upcast(
            NextServerUtilityModuleReference::new(Vc::upcast(*self.module))
                .to_resolved()
                .await?,
        )]))
    }

    #[turbo_tasks::function]
    fn side_effects(self: Vc<Self>) -> Vc<ModuleSideEffects> {
        // This just exports another import
        ModuleSideEffects::ModuleEvaluationIsSideEffectFree.cell()
    }
}

#[turbo_tasks::value_impl]
impl ChunkableModule for NextServerUtilityModule {
    #[turbo_tasks::function]
    fn as_chunk_item(
        self: ResolvedVc<Self>,
        module_graph: ResolvedVc<ModuleGraph>,
        chunking_context: ResolvedVc<Box<dyn ChunkingContext>>,
    ) -> Vc<Box<dyn turbopack_core::chunk::ChunkItem>> {
        Vc::upcast(
            NextServerComponentChunkItem {
                module_graph,
                chunking_context,
                inner: self,
            }
            .cell(),
        )
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkPlaceable for NextServerUtilityModule {
    #[turbo_tasks::function]
    fn get_exports(&self) -> Vc<EcmascriptExports> {
        let module_reference: Vc<Box<dyn ModuleReference>> = Vc::upcast(
            NextServerUtilityModuleReference::new(Vc::upcast(*self.module)),
        );
        EsmExports::reexport_including_default(module_reference)
    }
}

#[turbo_tasks::value]
struct NextServerComponentChunkItem {
    module_graph: ResolvedVc<ModuleGraph>,
    chunking_context: ResolvedVc<Box<dyn ChunkingContext>>,
    inner: ResolvedVc<NextServerUtilityModule>,
}

#[turbo_tasks::value_impl]
impl OutputAssetsReference for NextServerComponentChunkItem {}

#[turbo_tasks::value_impl]
impl EcmascriptChunkItem for NextServerComponentChunkItem {
    #[turbo_tasks::function]
    async fn content(&self) -> Result<Vc<EcmascriptChunkItemContent>> {
        let inner = self.inner.await?;

        let module_id = inner.module.chunk_item_id(*self.chunking_context).await?;
        Ok(EcmascriptChunkItemContent {
            inner_code: formatdoc!(
                "{TURBOPACK_EXPORT_NAMESPACE}({TURBOPACK_IMPORT}({}));",
                StringifyJs(&module_id),
            )
            .into(),
            ..Default::default()
        }
        .cell())
    }
}

#[turbo_tasks::value_impl]
impl ChunkItem for NextServerComponentChunkItem {
    #[turbo_tasks::function]
    fn asset_ident(&self) -> Vc<AssetIdent> {
        self.inner.ident()
    }

    #[turbo_tasks::function]
    fn chunking_context(&self) -> Vc<Box<dyn ChunkingContext>> {
        *self.chunking_context
    }

    #[turbo_tasks::function]
    fn ty(&self) -> Vc<Box<dyn ChunkType>> {
        Vc::upcast(Vc::<EcmascriptChunkType>::default())
    }

    #[turbo_tasks::function]
    fn module(&self) -> Vc<Box<dyn Module>> {
        Vc::upcast(*self.inner)
    }
}
