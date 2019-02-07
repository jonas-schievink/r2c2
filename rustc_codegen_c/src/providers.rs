//! Contains providers for a few rustc queries.
//!
//! Ideally, most of this stuff would be moved into other rustc crates like
//! `rustc_codegen_utils` to reduce duplication - most of this is based on the
//! providers in the LLVM codegen backend. In fact, we *must* register providers
//! for the same queries that the LLVM backend provides.

use rustc::hir::def_id::{DefId, LOCAL_CRATE};
use rustc::ty;
use rustc_data_structures::sync::Lrc;

pub fn provide(providers: &mut ty::query::Providers) {
    // from librustc_codegen_llvm/attributes.rs
    providers.target_features_whitelist = |tcx, cnum| {
        assert_eq!(cnum, LOCAL_CRATE);
        if tcx.sess.opts.actually_rustdoc {
            // rustdoc needs to be able to document functions that use all the features, so
            // whitelist them all
            // FIXME: This seems incompatible with other codegen backends: They don't know which
            // features LLVM supports
            Lrc::new(Default::default())
        } else {
            // TODO: figure this out - we should support the same features as the llvm backend
            Lrc::new(Default::default())
        }
    };

    provide_extern(providers);
}

pub fn provide_extern(providers: &mut ty::query::Providers) {
    // FIXME the LLVM backend sets `providers.wasm_import_module_map` here, figure out if we have to
}
