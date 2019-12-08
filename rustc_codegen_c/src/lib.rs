//! A rustc code generation backend that outputs C code.

// Much of the code in this crate is copied from the reference LLVM backend
#![allow(warnings)] // FIXME remove once this does something
#![feature(rustc_private)]

#[macro_use]
extern crate rustc;
extern crate rustc_apfloat;
extern crate rustc_mir;
extern crate rustc_target;
#[macro_use]
extern crate rustc_data_structures;
extern crate rustc_codegen_ssa;
extern crate rustc_codegen_utils;
extern crate rustc_errors;
extern crate rustc_incremental;
extern crate rustc_index;
#[macro_use]
extern crate syntax;
extern crate serialize;
extern crate syntax_pos;

#[macro_use]
extern crate log;
extern crate ar;
extern crate cc;
extern crate hashbrown;
extern crate object;
extern crate tempfile;
extern crate toolshed;
#[cfg(test)]
#[macro_use]
extern crate insta;
extern crate bitflags;

mod builder;
mod codegen;
mod metadata;
mod providers;
mod utils;

use rustc::dep_graph::{DepGraph, WorkProduct};
use rustc::hir::def_id::CrateNum;
use rustc::middle::cstore::{
    CrateSource, EncodedMetadata, LibSource, MetadataLoader, NativeLibrary,
};
use rustc::middle::lang_items::LangItem;
use rustc::session::config::{OptLevel, OutputFilenames, OutputType, PrintRequest};
use rustc::session::Session;
use rustc::ty::{self, TyCtxt};
use rustc::util::common::ErrorReported;
use rustc::util::nodemap::{FxHashMap, FxHashSet};
use rustc_codegen_ssa::back::lto::{LtoModuleCodegen, SerializedModule, ThinModule};
use rustc_codegen_ssa::back::write::{CodegenContext, FatLTOInput, ModuleConfig};
use rustc_codegen_ssa::traits::{
    ExtraBackendMethods, ModuleBufferMethods, ThinBufferMethods, WriteBackendMethods,
};
use rustc_codegen_ssa::{CompiledModule, ModuleCodegen};
use rustc_codegen_utils::codegen_backend::CodegenBackend;
use rustc_errors::{FatalError, Handler};
use rustc_mir::monomorphize;
use std::any::Any;
use std::panic;
use std::sync::mpsc;
use std::sync::Arc;
use syntax::expand::allocator::AllocatorKind;
use syntax::symbol::Symbol;

/// This is the entrypoint for a hot plugged rustc codegen backend.
#[no_mangle]
pub fn __rustc_codegen_backend() -> Box<dyn CodegenBackend> {
    Box::new(CCodegenBackend::new())
}

/// A codegen backend that translates the Rust codegen unit to C code and
/// invokes the system's C compiler to compile it.
#[derive(Clone, Copy)]
struct CCodegenBackend {}

impl CCodegenBackend {
    fn new() -> Self {
        Self {}
    }
}

impl CodegenBackend for CCodegenBackend {
    fn init(&self, sess: &Session) {
        // TODO (rustc): Try using `panic::set_hook` for ICE reporting so we can change it in here
        //panic::take_hook();
    }

    fn print(&self, req: PrintRequest, sess: &Session) {
        // FIXME: This is a dummy implementation
        match req {
            PrintRequest::RelocationModels => {
                println!("Available relocation models:");
                println!("    {}", "default");
            }
            PrintRequest::CodeModels => {
                println!("Available code models:");
                println!("    {}", "small");
            }
            PrintRequest::TlsModels => {
                println!("Available TLS models:");
                println!("    {}", "global-dynamic"); // I assume?
            }
            PrintRequest::TargetCPUs => {
                println!("Available CPUs for this target:");
                println!("    native         - Select the CPU of the current host");
            }
            PrintRequest::TargetFeatures => {
                println!("Available features for this target:");
                println!("    <none>");
            }
            _ => unimplemented!("print request {:?}", req),
        }
    }

    fn print_passes(&self) {
        unimplemented!("list of passes supported by the C compiler");
        // TODO: Parse output of `gcc --help=optimizers` and `clang --help`, resp.
    }

    fn print_version(&self) {
        println!(
            "{} version {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );

        match utils::cc_version() {
            Ok(out) => println!("{}", out),
            Err(err) => eprintln!("failed to determine C compiler version: {}", err),
        }
    }

    fn target_features(&self, sess: &Session) -> Vec<Symbol> {
        // TODO
        vec![]
    }

    fn metadata_loader(&self) -> Box<dyn MetadataLoader + Sync> {
        Box::new(metadata::Loader::new())
    }

    fn provide(&self, providers: &mut ty::query::Providers) {
        rustc_codegen_utils::symbol_names::provide(providers);
        rustc_codegen_ssa::back::symbol_export::provide(providers);
        rustc_codegen_ssa::base::provide_both(providers);

        providers::provide(providers);
    }

    fn provide_extern(&self, providers: &mut ty::query::Providers) {
        rustc_codegen_ssa::back::symbol_export::provide_extern(providers);
        rustc_codegen_ssa::base::provide_both(providers);

        providers::provide_extern(providers);
    }

    fn codegen_crate<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        metadata: EncodedMetadata,
        need_metadata_module: bool,
    ) -> Box<dyn Any> {
        // Let `rustc_codegen_ssa` do this. For this to work we have to
        // implement loads of traits from there (see below).
        Box::new(rustc_codegen_ssa::base::codegen_crate(
            Self {},
            tcx,
            metadata,
            need_metadata_module,
        ))
    }

    fn join_codegen_and_link(
        &self,
        ongoing_codegen: Box<dyn Any>,
        sess: &Session,
        dep_graph: &DepGraph,
        outputs: &OutputFilenames,
    ) -> Result<(), ErrorReported> {
        /*use rustc::util::common::time;
        let (ongoing_codegen, work_products) =
            ongoing_codegen.downcast::<::back::write::OngoingCodegen>()
                .expect("Expected LlvmCodegenBackend's OngoingCodegen, found Box<Any>")
                .join(sess);
        if sess.opts.debugging_opts.incremental_info {
            back::write::dump_incremental_data(&ongoing_codegen);
        }

        time(sess,
             "serialize work products",
             move || rustc_incremental::save_work_product_index(sess, &dep_graph, work_products));

        sess.compile_status()?;

        if !sess.opts.output_types.keys().any(|&i| i == OutputType::Exe ||
            i == OutputType::Metadata) {
            return Ok(());
        }

        // Run the linker on any artifacts that resulted from the LLVM run.
        // This should produce either a finished executable or library.
        time(sess, "linking", || {
            back::link::link_binary(sess, &ongoing_codegen,
                                    outputs, &ongoing_codegen.crate_name.as_str());
        });

        // Now that we won't touch anything in the incremental compilation directory
        // any more, we can finalize it (which involves renaming it)
        rustc_incremental::finalize_session_directory(sess, ongoing_codegen.link.crate_hash);

        Ok(())*/
        unimplemented!("join_codegen_and_link")
    }
}

// Implement all the rustc_codegen_ssa traits

impl WriteBackendMethods for CCodegenBackend {
    type Module = codegen::Module;
    type TargetMachine = ();
    type ModuleBuffer = NoModuleBuffer;
    type Context = ();
    type ThinData = ();
    type ThinBuffer = NoThinBuffer;

    fn run_fat_lto(
        cgcx: &CodegenContext<Self>,
        modules: Vec<FatLTOInput<Self>>,
        cached_modules: Vec<(SerializedModule<Self::ModuleBuffer>, WorkProduct)>,
    ) -> Result<LtoModuleCodegen<Self>, FatalError> {
        unimplemented!("C backend fat LTO")
    }

    fn run_thin_lto(
        cgcx: &CodegenContext<Self>,
        modules: Vec<(String, Self::ThinBuffer)>,
        cached_modules: Vec<(SerializedModule<Self::ModuleBuffer>, WorkProduct)>,
    ) -> Result<(Vec<LtoModuleCodegen<Self>>, Vec<WorkProduct>), FatalError> {
        unimplemented!("C backend thin LTO")
    }

    fn print_pass_timings(&self) {
        unimplemented!("print_pass_timings");
    }

    unsafe fn optimize(
        cgcx: &CodegenContext<Self>,
        diag_handler: &Handler,
        module: &ModuleCodegen<Self::Module>,
        config: &ModuleConfig,
    ) -> Result<(), FatalError> {
        unimplemented!()
    }

    unsafe fn optimize_thin(
        cgcx: &CodegenContext<Self>,
        thin: &mut ThinModule<Self>,
    ) -> Result<ModuleCodegen<Self::Module>, FatalError> {
        unimplemented!()
    }

    unsafe fn codegen(
        cgcx: &CodegenContext<Self>,
        diag_handler: &Handler,
        module: ModuleCodegen<Self::Module>,
        config: &ModuleConfig,
    ) -> Result<CompiledModule, FatalError> {
        unimplemented!()
    }

    fn prepare_thin(module: ModuleCodegen<Self::Module>) -> (String, Self::ThinBuffer) {
        unimplemented!("C backend ThinLTO")
    }

    fn run_lto_pass_manager(
        cgcx: &CodegenContext<Self>,
        llmod: &ModuleCodegen<Self::Module>,
        config: &ModuleConfig,
        thin: bool,
    ) {
        unimplemented!("C backend LTO")
    }

    fn serialize_module(module: ModuleCodegen<Self::Module>) -> (String, Self::ModuleBuffer) {
        unimplemented!("C backend serialize_module")
    }
}

impl ExtraBackendMethods for CCodegenBackend {
    /// Create a new `Module` for storing metadata.
    fn new_metadata(&self, sess: TyCtxt, mod_name: &str) -> Self::Module {
        //unimplemented!("new_metadata")
        codegen::Module
    }

    fn write_compressed_metadata<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        metadata: &EncodedMetadata,
        llvm_module: &mut Self::Module,
    ) {
        unimplemented!();
    }

    fn codegen_allocator(&self, tcx: TyCtxt, mods: &mut Self::Module, kind: AllocatorKind) {
        unimplemented!("codegen_allocator")
    }

    fn compile_codegen_unit<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        cgu_name: Symbol,
        tx_to_llvm_workers: &mpsc::Sender<Box<dyn Any + Send>>,
    ) {
        unimplemented!()
    }

    fn target_machine_factory(
        &self,
        sess: &Session,
        opt_level: OptLevel,
        find_features: bool,
    ) -> Arc<Fn() -> Result<Self::TargetMachine, String> + Send + Sync> {
        Arc::new(|| Err("unimplemented".to_string()))
    }

    fn target_cpu<'b>(&self, sess: &'b Session) -> &'b str {
        unimplemented!()
    }
}

struct NoModuleBuffer;

impl ModuleBufferMethods for NoModuleBuffer {
    fn data(&self) -> &[u8] {
        &[]
    }
}

struct NoThinBuffer;

impl ThinBufferMethods for NoThinBuffer {
    fn data(&self) -> &[u8] {
        &[]
    }
}
