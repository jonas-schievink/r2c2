//! Metadata loader using the `ar` and `object` crates.

use utils::ResultExt;

use ar::Archive;
use object::{File as ObjectFile, Object, ObjectSection};

use rustc::hir::def_id::LOCAL_CRATE;
use rustc::middle::cstore::EncodedMetadata;
use rustc::middle::cstore::MetadataLoader;
use rustc::session::config::CrateType;
use rustc::ty::TyCtxt;
use rustc_data_structures::owning_ref::OwningRef;
use rustc_data_structures::sync::MetadataRef;
use rustc_target::spec::Target;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

/// Metadata filename for metadata stored in an rlib.
const METADATA_FILENAME: &str = "lib.rmeta";

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
    fn get_rlib_metadata(&self, target: &Target, file: &Path) -> Result<MetadataRef, String> {
        match get_rlib_metadata(target, file) {
            Ok(meta) => Ok(meta),
            Err(e) => {
                error!("get_rlib_metadata: {}", e);
                Err(e)
            }
        }
    }

    fn get_dylib_metadata(&self, target: &Target, file: &Path) -> Result<MetadataRef, String> {
        match get_dylib_metadata(target, file) {
            Ok(meta) => Ok(meta),
            Err(e) => {
                error!("get_dylib_metadata: {}", e);
                Err(e)
            }
        }
    }
}

fn get_rlib_metadata(_target: &Target, filename: &Path) -> Result<MetadataRef, String> {
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

fn get_dylib_metadata(_target: &Target, file: &Path) -> Result<MetadataRef, String> {
    // A dylib is a platform-dependent shared library (on Linux, a `.so`
    // file). The metadata is stored in the `.rustc` section. Use the
    // `object` crate to extract it.

    info!("reading dylib metadata from {}", file.display());
    let data = fs::read(file).err_to_string()?;
    let obj = ObjectFile::parse(&data).err_to_string()?;

    let metadata_section = obj.section_by_name(METADATA_SECTION_NAME).ok_or_else(|| {
        format!(
            "couldn't find dylib metadata section {} in {}",
            METADATA_SECTION_NAME,
            file.display()
        )
    })?;

    let content = metadata_section.data();
    let data: OwningRef<_, [u8]> =
        OwningRef::new(content.into_owned().into_boxed_slice()).map_owner_box();
    return Ok(rustc_erase_owner!(data));
}

/// Encode metadata into an appropriate blob for a given `crate_type`.
///
/// This will also perform any compression required by the crate type.
///
/// Returns `None` if the `crate_type` doesn't carry metadata (eg.
/// executables, C-compatible libs).
pub fn encode(tcx: TyCtxt<'_>) -> EncodedMetadata {
    tcx.encode_metadata()
}
