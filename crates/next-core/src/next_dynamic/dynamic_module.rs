use anyhow::Result;
use indoc::formatdoc;
use turbo_rcstr::{RcStr, rcstr};
use turbo_tasks::{ResolvedVc, Vc};
use turbopack_core::{
    chunk::{ChunkItem, ChunkType, ChunkableModule, ChunkingContext, ModuleChunkItemIdExt},
    ident::AssetIdent,
    module::{Module, ModuleSideEffects},
    module_graph::ModuleGraph,
    output::OutputAssetsReference,
    reference::{ModuleReference, ModuleReferences, SingleChunkableModuleReference},
    resolve::ExportUsage,
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

/// A [`NextDynamicEntryModule`] is a marker asset used to indicate which
/// dynamic assets should appear in the dynamic manifest.
#[turbo_tasks::value(shared)]
pub struct NextDynamicEntryModule {
    module: ResolvedVc<Box<dyn EcmascriptChunkPlaceable>>,
}

#[turbo_tasks::value_impl]
impl NextDynamicEntryModule {
    #[turbo_tasks::function]
    pub fn new(module: ResolvedVc<Box<dyn EcmascriptChunkPlaceable>>) -> Vc<Self> {
        NextDynamicEntryModule { module }.cell()
    }
}

fn dynamic_ref_description() -> RcStr {
    rcstr!("next/dynamic reference")
}

#[turbo_tasks::value_impl]
impl Module for NextDynamicEntryModule {
    #[turbo_tasks::function]
    fn ident(&self) -> Vc<AssetIdent> {
        self.module
            .ident()
            .with_modifier(rcstr!("next/dynamic entry"))
    }

    #[turbo_tasks::function]
    fn source(&self) -> Vc<OptionSource> {
        Vc::cell(None)
    }

    #[turbo_tasks::function]
    async fn references(&self) -> Result<Vc<ModuleReferences>> {
        Ok(Vc::cell(vec![ResolvedVc::upcast(
            SingleChunkableModuleReference::new(
                Vc::upcast(*self.module),
                dynamic_ref_description(),
                ExportUsage::all(),
            )
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
impl ChunkableModule for NextDynamicEntryModule {
    #[turbo_tasks::function]
    fn as_chunk_item(
        self: ResolvedVc<Self>,
        module_graph: ResolvedVc<ModuleGraph>,
        chunking_context: ResolvedVc<Box<dyn ChunkingContext>>,
    ) -> Vc<Box<dyn turbopack_core::chunk::ChunkItem>> {
        Vc::upcast(
            NextDynamicEntryChunkItem {
                chunking_context,
                module_graph,
                inner: self,
            }
            .cell(),
        )
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkPlaceable for NextDynamicEntryModule {
    #[turbo_tasks::function]
    fn get_exports(&self) -> Vc<EcmascriptExports> {
        let module_reference: Vc<Box<dyn ModuleReference>> =
            Vc::upcast(SingleChunkableModuleReference::new(
                Vc::upcast(*self.module),
                dynamic_ref_description(),
                ExportUsage::all(),
            ));
        EsmExports::reexport(module_reference)
    }
}

#[turbo_tasks::value]
struct NextDynamicEntryChunkItem {
    chunking_context: ResolvedVc<Box<dyn ChunkingContext>>,
    module_graph: ResolvedVc<ModuleGraph>,
    inner: ResolvedVc<NextDynamicEntryModule>,
}

#[turbo_tasks::value_impl]
impl OutputAssetsReference for NextDynamicEntryChunkItem {}

#[turbo_tasks::value_impl]
impl EcmascriptChunkItem for NextDynamicEntryChunkItem {
    #[turbo_tasks::function]
    async fn content(&self) -> Result<Vc<EcmascriptChunkItemContent>> {
        let inner = self.inner.await?;

        let module_id = inner.module.chunk_item_id(*self.chunking_context).await?;
        Ok(EcmascriptChunkItemContent {
            inner_code: formatdoc!(
                r#"
                    {TURBOPACK_EXPORT_NAMESPACE}({TURBOPACK_IMPORT}({}));
                "#,
                StringifyJs(&module_id),
            )
            .into(),
            ..Default::default()
        }
        .cell())
    }
}

#[turbo_tasks::value_impl]
impl ChunkItem for NextDynamicEntryChunkItem {
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
