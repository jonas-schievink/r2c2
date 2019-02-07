//! Metadata loader using the `ar` and `object` crates.

use utils::ResultExt;

use ar::Archive;

use rustc::hir::def_id::LOCAL_CRATE;
use rustc::middle::cstore::MetadataLoader;
use rustc::session::config::CrateType;
use rustc::ty::TyCtxt;
use rustc_data_structures::owning_ref::OwningRef;
use rustc_data_structures::sync::MetadataRef;
use rustc_target::spec::Target;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use rustc::middle::cstore::EncodedMetadata;

/// Metadata filename for metadata stored in an rlib.
const METADATA_FILENAME: &str = "rust.metadata.bin";
/// Section name for metadata stored in a dylib.
const METADATA_SECTION_NAME: &str = ".rustc";

/// Metadata loader.
///
/// The LLVM backend uses LLVM's own archive and object reading functionality.
/// We use the `ar` and `object` crates, so this metadata loader should be
/// usable from any other codegen backend.
pub struct Loader {}

impl Loader {
    pub fn new() -> Self {
        Self {}
    }
}

impl MetadataLoader for Loader {
    fn get_rlib_metadata(&self, _: &Target, filename: &Path) -> Result<MetadataRef, String> {
        // A `.rlib` file is just a `.a`-archive that contains the metadata in a
        // `rust.metadata.bin` file. Use the `ar` crate to read that file.

        info!(
            "loading rlib metadata from archive file '{}'",
            filename.display()
        );
        let mut archive = Archive::new(File::open(filename).err_to_string()?);

        while let Some(entry) = archive.next_entry() {
            let mut entry = entry.err_to_string()?;
            if entry.header().identifier() == METADATA_FILENAME.as_bytes() {
                let mut content: Vec<u8> = Vec::new();
                entry.read_to_end(&mut content).err_to_string()?;

                let data: OwningRef<_, [u8]> =
                    OwningRef::new(content.into_boxed_slice()).map_owner_box();
                return Ok(rustc_erase_owner!(data));
            }
        }

        Err(format!(
            "couldn't find metadata file {} in rlib '{}'",
            METADATA_FILENAME,
            filename.display()
        ))
    }

    fn get_dylib_metadata(
        &self,
        _target: &Target,
        _filename: &Path,
    ) -> Result<MetadataRef, String> {
        // A dylib is a platform-dependent shared library (on Linux, a `.so`
        // file). The metadata is stored in the `.rustc` section. Use the
        // `object` crate to extract it.
        unimplemented!()
    }
}

/// Encode metadata into an appropriate blob for a given `crate_type`.
///
/// This will also perform any compression required by the crate type.
///
/// Returns `None` if the `crate_type` doesn't carry metadata (eg.
/// executables, C-compatible libs).
pub fn encode<'a, 'tcx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
) -> EncodedMetadata {
    tcx.encode_metadata()
}
