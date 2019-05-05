//! Function builder.

use super::types::{FnSig, TypeRef};
use super::Symbol;
use utils::WriteStr;

use hashbrown::HashSet;
use toolshed::Arena;

use std::io;

/// A local variable.
///
/// These can be created in 2 ways: By declaring a function taking arguments
/// (which creates `Variable`s corresponding to the arguments), or by calling
/// `declare_variable` on an existing `FunctionBuilder`, which declares a new
/// local variable inside the function.
#[derive(Debug, Copy, Clone)]
pub struct Variable<'a> {
    name: &'a str,
    ty: TypeRef<'a>,
}

/// Builder for function bodies.
///
/// Created by `TranslationUnitBuilder::define_function`.
pub struct FunctionBuilder<'a, W: WriteStr> {
    /// Output statements are written to this writer.
    writer: &'a mut W,
    arena: &'a Arena,
    declared_locals: HashSet<String>,
    finished: bool,

    /// Function arguments, available as declared `Variable`s.
    pub args: &'a [Variable<'a>],
}

impl<'a, W: WriteStr> FunctionBuilder<'a, W> {
    /// Creates a function builder, writing the function's header to `writer`.
    ///
    /// **Note**: You *must* call `finish` on the builder to close the function
    /// being built, or this will panic on drop.
    pub fn create(
        writer: &'a mut W,
        arena: &'a Arena,
        name: Symbol<'_>,
        proto: FnSig<'a>,
    ) -> io::Result<Self> {
        proto.declare(name.mangled(), writer)?;
        writeln!(writer)?;
        writeln!(writer, "{{")?;
        Ok(Self {
            writer,
            arena,
            declared_locals: HashSet::new(),
            finished: false,
            args: arena.alloc_slice(
                &proto
                    .args
                    .iter()
                    .enumerate()
                    .map(|(i, &ty)| Variable {
                        name: arena.alloc_str(&format!("_{}", i)),
                        ty,
                    })
                    .collect::<Vec<_>>(),
            ),
        })
    }

    /// Closes the in-progress function definition, consuming `self`.
    pub fn finish(mut self) -> io::Result<()> {
        writeln!(self.writer, "}}")?;
        writeln!(self.writer)?;
        self.finished = true;
        Ok(())
    }

    fn indent(&mut self) -> io::Result<()> {
        write!(self.writer, "    ")
    }

    /// Declares a new local variable at the current position.
    pub fn declare_variable<'b>(
        &mut self,
        name: impl AsRef<str>,
        ty: TypeRef<'a>,
        comment: impl Into<Option<&'b str>>,
    ) -> io::Result<Variable<'a>> {
        let name = name.as_ref();
        self.indent()?;
        ty.declare_variable(name, self.writer)?;
        write!(self.writer, ";")?;
        if let Some(comment) = comment.into() {
            write!(self.writer, "  /* {} */", comment)?;
        }
        writeln!(self.writer)?;

        Ok(Variable {
            name: self.arena.alloc_str(name),
            ty,
        })
    }
}

impl<'a, W: WriteStr> Drop for FunctionBuilder<'a, W> {
    fn drop(&mut self) {
        if !self.finished {
            panic!("FunctionBuilder dropped without calling `finish`");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use builder::test::compile_test;

    #[test]
    fn declare_locals() {
        compile_test("declare_locals", |tu| {
            let a = Arena::new();
            let sig = tu.fn_sig(None, &[]);
            let double = tu.double();
            let fnptr = tu.fn_ptr(sig);
            let mut f = tu.define_function(&a, Symbol::test("declare_locals"), sig)?;
            f.declare_variable("dbl", double, None)?;
            f.declare_variable("f", fnptr, Some("i'm a function pointer with a comment"))?;
            f.finish()?;
            Ok(())
        });
    }
}
