//! Custom ICE panic hook.

use git_version::git_version;
use once_cell::sync::Lazy;

use rustc::ty::TyCtxt;
use std::borrow::Cow;
use std::{panic, thread};

const BUG_REPORT_URL: &str = "https://github.com/jonas-schievink/r2c2";

pub fn register_hook() {
    let old_hook = panic::take_hook();
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| report_ice(info, &default_hook)));
}

fn report_ice(
    info: &panic::PanicInfo<'_>,
    default_hook: &(dyn Fn(&panic::PanicInfo<'_>) + Sync + Send + 'static),
) {
    eprintln!();
    (default_hook)(info);
    eprintln!();

    let emitter = Box::new(rustc_errors::emitter::EmitterWriter::stderr(
        rustc_errors::ColorConfig::Auto,
        None,
        false,
        false,
        None,
        false,
    ));
    let handler = rustc_errors::Handler::with_emitter(true, None, emitter);

    // a .span_bug or .bug call has already printed what
    // it wants to print.
    if !info.payload().is::<rustc_errors::ExplicitBug>() {
        let d = rustc_errors::Diagnostic::new(rustc_errors::Level::Bug, "unexpected panic");
        handler.emit_diagnostic(&d);
        handler.abort_if_errors_and_should_abort();
    }

    let xs: Vec<Cow<'static, str>> = vec![
        "the compiler unexpectedly panicked. this is a bug.".into(),
        format!("we would appreciate a bug report: {}", BUG_REPORT_URL).into(),
        format!("r2c2 commit: {}", git_version!()).into(),
    ];

    for note in &xs {
        handler.note_without_error(&note);
    }

    // If backtraces are enabled, also print the query stack
    let backtrace = std::env::var_os("RUST_BACKTRACE").map_or(false, |x| &x != "0");

    if backtrace {
        TyCtxt::try_print_query_stack(&handler);
    }
}
