use rustc::ty::TyCtxt;
use std::fmt::{self, Display};
use std::io::{self, Write};
use std::ops::Deref;
use std::ops::DerefMut;
use std::str;

// Import the host/target triple, set by the build script
include!(concat!(env!("OUT_DIR"), "/triples.rs"));

/// Extension methods for `Result<T, E>`.
pub trait ResultExt<T, E> {
    /// Turns the error into a `String`.
    fn err_to_string(self) -> Result<T, String>
    where
        E: ToString;
}

impl<T, E: Display> ResultExt<T, E> for Result<T, E> {
    fn err_to_string(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

/// Obtains the version information of the system's C compiler.
///
/// TODO: Upstream this to the `cc` crate
pub fn cc_version() -> Result<String, String> {
    // FIXME: cc::Error doesn't impl Display and Error
    let mut compiler = cc::Build::new()
        .try_get_compiler()
        .map_err(|e| format!("{:?}", e))?
        .to_command();
    let output = compiler.arg("--version").output().err_to_string()?;

    let mut out = String::from_utf8(output.stdout).err_to_string()?;
    out.push_str(str::from_utf8(&output.stderr).err_to_string()?);

    if output.status.success() {
        Ok(out)
    } else {
        Err(out)
    }
}

/// Trait for writing UTF-8 data to a sink.
pub trait WriteStr {
    fn write_fmt(&mut self, args: fmt::Arguments) -> io::Result<()>;

    fn write_str(&mut self, s: &str) -> io::Result<()> {
        write!(self, "{}", s)
    }
}

impl<W: Write> WriteStr for W {
    fn write_fmt(&mut self, args: fmt::Arguments) -> io::Result<()> {
        <W as Write>::write_fmt(self, args)
    }
}

pub struct StringWriter(pub String);

impl WriteStr for StringWriter {
    fn write_fmt(&mut self, args: fmt::Arguments) -> io::Result<()> {
        Ok(<String as fmt::Write>::write_fmt(&mut self.0, args).expect("writing to string failed"))
    }
}

impl Deref for StringWriter {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl DerefMut for StringWriter {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl AsRef<str> for StringWriter {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
