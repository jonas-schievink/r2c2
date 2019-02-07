//! Contains code generation for the global allocator.
//!
//! This is basically glue code that defines `__rust_alloc`-style functions and
//! forwards them to the correct global allocator (global allocator, and default
//! allocators for libraries and binaries).
//!
//! The code in rustc isn't really documented at all, so this is mostly
//! speculation and a port of the LLVM backend's code.
