//! Wrapper around `rustc` that uses the C codegen backend.

#[macro_use]
extern crate log;
extern crate env_logger;

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{exit, Command};

fn locate_codegen_library() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push(if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }); // (heuristic)
    path.push("deps"); // because it's a dependency of this wrapper crate
    path.push("librustc_codegen_c.so"); // FIXME: macOS/Windows?
    path
}

fn main() {
    env_logger::init();

    let backend_path = locate_codegen_library();
    if !backend_path.exists() {
        eprintln!(
            "couldn't locate C codegen backend (path doesn't exist: {})",
            backend_path.display()
        );
    }

    info!("using C codegen backend at {}", backend_path.display());

    let rustc = env::var_os("RUSTC").unwrap_or(OsString::from("rustc"));
    info!("using rust compiler '{}'", rustc.to_string_lossy());

    // forward our args to rustc
    let mut args: Vec<_> = env::args_os().skip(1).collect();

    // append `-Zcodegen-backend` (it doesn't have to come before the file name)
    let mut path_str = OsString::from("-Zcodegen-backend=");
    path_str.push(backend_path.as_os_str());
    args.push(path_str);

    let mut cmd = Command::new(rustc);
    cmd.args(args);
    let status = cmd.status().expect("couldn't execute rustc");

    let code = status.code().unwrap_or(-1); // return -1 on any signals for now
    exit(code);
}
