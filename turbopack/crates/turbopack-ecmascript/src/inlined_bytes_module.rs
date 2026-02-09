use std::io::{Read, Write};

use anyhow::{Result, bail};
use turbo_rcstr::rcstr;
use turbo_tasks::{ResolvedVc, ValueToString, Vc};
use turbo_tasks_fs::{FileContent, rope::RopeBuilder};
use turbopack_core::{
    asset::Asset,
    chunk::{AsyncModuleInfo, ChunkableModule, ChunkingContext},
    ident::AssetIdent,
    module::{Module, ModuleSideEffects},
    module_graph::ModuleGraph,
    source::Source,
};

use crate::{
    chunk::{
        EcmascriptChunkItemContent, EcmascriptChunkPlaceable, EcmascriptExports,
        ecmascript_chunk_item,
    },
    runtime_functions::TURBOPACK_EXPORT_VALUE,
    utils::StringifyJs,
};

#[turbo_tasks::value]
pub struct InlinedBytesJsModule {
    source: ResolvedVc<Box<dyn Source>>,
}

#[turbo_tasks::value_impl]
impl InlinedBytesJsModule {
    #[turbo_tasks::function]
    pub fn new(source: ResolvedVc<Box<dyn Source>>) -> Vc<Self> {
        Self::cell(InlinedBytesJsModule { source })
    }
}

#[turbo_tasks::value_impl]
impl Module for InlinedBytesJsModule {
    #[turbo_tasks::function]
    fn ident(&self) -> Vc<AssetIdent> {
        self.source
            .ident()
            .with_modifier(rcstr!("static bytes in ecmascript"))
    }

    #[turbo_tasks::function]
    fn source(&self) -> Vc<turbopack_core::source::OptionSource> {
        Vc::cell(Some(self.source))
    }

    #[turbo_tasks::function]
    fn side_effects(self: Vc<Self>) -> Vc<ModuleSideEffects> {
        ModuleSideEffects::SideEffectFree.cell()
    }
}

#[turbo_tasks::value_impl]
impl ChunkableModule for InlinedBytesJsModule {
    #[turbo_tasks::function]
    fn as_chunk_item(
        self: ResolvedVc<Self>,
        module_graph: Vc<ModuleGraph>,
        chunking_context: ResolvedVc<Box<dyn ChunkingContext>>,
    ) -> Vc<Box<dyn turbopack_core::chunk::ChunkItem>> {
        ecmascript_chunk_item(Vc::upcast(*self), module_graph, *chunking_context)
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkPlaceable for InlinedBytesJsModule {
    #[turbo_tasks::function]
    fn get_exports(&self) -> Vc<EcmascriptExports> {
        EcmascriptExports::Value.cell()
    }

    #[turbo_tasks::function]
    async fn chunk_item_content(
        self: Vc<Self>,
        _chunking_context: Vc<Box<dyn ChunkingContext>>,
        _module_graph: Vc<ModuleGraph>,
        _async_module_info: Option<Vc<AsyncModuleInfo>>,
        _estimated: bool,
    ) -> Result<Vc<EcmascriptChunkItemContent>> {
        let this = self.await?;
        let content = this.source.content().file_content().await?;
        match &*content {
            FileContent::Content(data) => {
                let mut inner_code = RopeBuilder::default();
                inner_code += "
var decode = Uint8Array.fromBase64 || function Uint8Array_fromBase64(base64) {
  var binaryString = atob(base64);
  var buffer = new Uint8Array(binaryString.length);
  for (var i = 0; i < binaryString.length; i++) {
    buffer[i] = binaryString.charCodeAt(i)
  }
  return buffer
};\n";

                let encoded = data_encoding::BASE64_NOPAD
                    .encode(&data.read().bytes().collect::<std::io::Result<Vec<u8>>>()?);
                write!(
                    inner_code,
                    "{TURBOPACK_EXPORT_VALUE}(decode({}));",
                    StringifyJs(&encoded)
                )?;

                Ok(EcmascriptChunkItemContent {
                    inner_code: inner_code.build(),
                    ..Default::default()
                }
                .cell())
            }
            FileContent::NotFound => {
                bail!("File not found: {}", self.ident().to_string().await?);
            }
        }
    }
}
