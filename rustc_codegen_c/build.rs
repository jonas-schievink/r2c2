use std::env;
use std::fs::File;
use std::io::Write;

fn main() {
    let out = format!("{}/triples.rs", env::var("OUT_DIR").unwrap());
    println!("out: {}", out);

    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();

    let mut file = File::create(out).unwrap();
    writeln!(file, "pub const TARGET: &str = \"{}\";", target).unwrap();
    writeln!(file, "pub const HOST: &str = \"{}\";", host).unwrap();

    // only rerun if the script changes (otherwise, this would be rerun *every*
    // build and cause a recompilation *every time*)
    println!("cargo:rerun-if-changed=build.rs");
}
