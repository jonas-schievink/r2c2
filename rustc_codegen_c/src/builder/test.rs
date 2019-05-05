//! Unit test utilities.

#![cfg(test)]

use super::function::FunctionBuilder;
use super::{FnSig, Symbol, TranslationUnitBuilder, TypeRef};
use utils::{StringWriter, WriteStr};

use cc::Build;
use std::error::Error;
use std::io::Write;
use tempfile::{self, TempDir};
use toolshed::Arena;

/// Result to return from unit tests (instead of panicking).
pub type TestResult = Result<(), Box<Error>>;

/// Calls the test function `f` with a `TranslationUnitBuilder` and tests
/// that the produced output can be compiled by the system's C compiler.
///
/// Returns the produced output as a `String` that can be used for a
/// snapshot test.
// TODO: remove this and replace with `compile_test` below
pub fn test_tu<F>(f: F) -> String
where
    F: FnOnce(&mut TranslationUnitBuilder<'_, StringWriter>) -> TestResult,
{
    let arena = Arena::new();
    let mut writer = StringWriter(String::new());
    let mut builder = TranslationUnitBuilder::create(&arena, writer).unwrap();

    f(&mut builder).unwrap();

    let dir = TempDir::new().unwrap();

    let mut f = tempfile::Builder::new().suffix(".c").tempfile().unwrap();
    f.write_all(&builder.writer().0.as_bytes()).unwrap();
    Build::new()
        .file(f.path())
        .cargo_metadata(false)
        .out_dir(&dir)
        .target(::utils::TARGET)
        .host(::utils::HOST)
        .opt_level(0)
        .compile("foo");
    builder.into_writer().0
}

pub fn compile_test<F>(name: &str, f: F)
where
    F: FnOnce(&mut TranslationUnitBuilder<'_, StringWriter>) -> TestResult,
{
    let arena = Arena::new();
    let mut writer = StringWriter(String::new());
    let mut builder = TranslationUnitBuilder::create(&arena, writer).unwrap();

    f(&mut builder).unwrap();

    let dir = TempDir::new().unwrap();

    let mut f = tempfile::Builder::new().suffix(".c").tempfile().unwrap();
    f.write_all(&builder.writer().0.as_bytes()).unwrap();
    Build::new()
        .file(f.path())
        .cargo_metadata(false)
        .out_dir(&dir)
        .target(::utils::TARGET)
        .host(::utils::HOST)
        .opt_level(0)
        .compile("foo");
    let output = builder.into_writer().0;
    assert_snapshot_matches!(name, output);
}
