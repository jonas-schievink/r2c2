//! The actual C code generation.

pub mod allocator;

use rustc::session::Session;
use rustc::ty::TyCtxt;
use rustc_codegen_utils::check_for_rustc_errors_attr;

use std::any::Any;
use std::sync::mpsc;

/// C codegen module.
pub struct Module;

/// Token for an ongoing code generation for a crate.
pub struct OngoingCodegen {}

/// Starts code generation for the current crate.
///
/// This can generate any number of artifacts (rlibs, dylibs, binaries) and
/// object files (each representing a codegen unit).
pub fn start_codegen_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    rx: mpsc::Receiver<Box<dyn Any + Send>>,
) -> OngoingCodegen {
    // abort if `#[rustc_error]` is used by tests - this could be done by rustc
    // just before codegen is invoked (FIXME)
    check_for_rustc_errors_attr(tcx);

    report_unsupported_flags(tcx.sess);

    // Skip crate items and just output metadata in -Z no-codegen mode.
    // FIXME: rustc can do this before invoking the codegen backend
    if tcx.sess.opts.debugging_opts.no_codegen || !tcx.sess.opts.output_types.should_codegen() {}

    OngoingCodegen {}
}

/// Checks for command line flags that aren't or cannot be supported by the C
/// backend and reports an error if any were found.
fn report_unsupported_flags(sess: &Session) {
    if let Some(true) = sess.opts.debugging_opts.thinlto {
        sess.fatal("the C codegen backend doesn't support ThinLTO");
    }

    if sess.opts.cg.profile_generate.enabled() || sess.opts.cg.profile_use.is_some() {
        sess.fatal("the C codegen backend doesn't support profile-guided optimization");
    }

    if sess.time_llvm_passes() {
        sess.fatal("-Ztime-llvm-passes not supported by C codegen backend");
    }

    if sess.verify_llvm_ir() {
        sess.fatal("-Zverify-llvm-ir not supported by C codegen backend");
    }

    if sess.print_llvm_passes() {
        sess.fatal("-Zprint-llvm-passes not supported by C codegen backend");
    }

    if sess.opts.debugging_opts.embed_bitcode {
        sess.fatal("-Zembed-bitcode not supported by C codegen backend");
    }

    if sess.opts.cg.linker_plugin_lto.enabled() {
        sess.fatal("linker plugin based LTO not supported by C codegen backend");
    }

    // TODO: incomplete
}
