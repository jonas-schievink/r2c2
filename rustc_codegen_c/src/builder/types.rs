//! C type definitions and combinators.
//!
//! Make sure all C types have to be created through the `Factory`.

use utils::{StringWriter, WriteStr};

use std::fmt;
use std::io::{self, Write};

/// A function signature.
#[derive(Copy, Clone)]
pub struct FnSig<'a> {
    pub ret: TypeRef<'a>,
    pub args: &'a [TypeRef<'a>],
}

impl FnSig<'_> {
    /// (Forward-)Declares a function with this signature (partially).
    ///
    /// The output will not contain a trailing `;`. This makes this function
    /// also useful for declaring function pointer variables/static and casts.
    pub fn declare(&self, name: impl Into<String>, w: &mut impl WriteStr) -> io::Result<()> {
        // this is perverse
        let mut buf = StringWriter(name.into());
        buf.push('(');
        for (i, arg) in self.args.iter().enumerate() {
            if i != 0 {
                buf.push_str(", ");
            }

            arg.declare_variable(format!("_{}", i + 1), &mut buf)?;
        }
        if self.args.is_empty() {
            // `void` ensures the compiler doesn't let us call the function with arguments
            buf.push_str("void");
        }
        buf.push(')');

        self.ret.declare_variable(&buf, w)
    }
}

#[derive(Copy, Clone)]
pub enum Type<'a> {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    Float,
    Double,
    Pointer(&'a Type<'a>),
    /// `intptr_t` aka `isize`.
    IntPtr,
    /// `uintptr_t` aka `usize`
    UintPtr,
    Array {
        ty: &'a Type<'a>,
        len: usize,
    },
    FunctionPointer(FnSig<'a>),
    Struct {
        name: &'a str,
    },

    // Incomplete types
    Void,
    /// A forward-declared struct with unspecified fields.
    FwdStruct {
        /// The struct's name (mangled Rust name).
        name: &'a str,
    },
    FwdUnion {
        name: &'a str,
    },
}

impl Type<'_> {
    fn declare_variable<W: WriteStr>(&self, name: &str, w: &mut W) -> io::Result<()> {
        let simplety = match self {
            Type::Bool => "bool",
            Type::U8 => "uint8_t",
            Type::I8 => "int8_t",
            Type::U16 => "uint16_t",
            Type::I16 => "int16_t",
            Type::U32 => "uint32_t",
            Type::I32 => "int32_t",
            Type::U64 => "uint64_t",
            Type::I64 => "int64_t",
            Type::Float => "float",
            Type::Double => "double",
            Type::IntPtr => "intptr_t",
            Type::UintPtr => "uintptr_t",
            Type::Pointer(pointee) => return pointee.declare_variable(&format!("* {}", name), w),
            Type::Array { ty, len } => {
                return ty.declare_variable(&format!("{}[{}]", name, len), w)
            }
            Type::FunctionPointer(sig) => {
                // fn pointer syntax is like fn declaration syntax except the variable name is put
                // in `(*NAME)` where the fn name is
                return sig.declare(format!("(*{})", name), w);
            }
            Type::FwdStruct { name: ty } | Type::Struct { name: ty } => {
                return write!(w, "struct {} {}", ty, name)
            }
            Type::FwdUnion { name: ty } => return write!(w, "union {} {}", ty, name),
            // the void case is needed to declare functions and fn pointers
            Type::Void => return write!(w, "void {}", name),
        };

        write!(w, "{} {}", simplety, name)
    }
}

impl fmt::Debug for Type<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = StringWriter(String::new());
        self.declare_variable("", &mut buf).unwrap();
        f.write_str(&buf)
    }
}

/// Trait abstracting over `TypeRef` and `IncompleteTypeRef`, used by methods
/// that can work with either.
pub trait AsType<'a> {
    fn as_type(&self) -> &'a Type<'a>;
}

/// Reference to a complete C type.
#[derive(Debug, Copy, Clone)]
pub struct TypeRef<'a>(pub &'a Type<'a>);

impl<'a> AsType<'a> for TypeRef<'a> {
    fn as_type(&self) -> &'a Type<'a> {
        self.0
    }
}

impl TypeRef<'_> {
    /// Declares a variable of this type and prints the declaration to `f`.
    ///
    /// Output is normally of the form `type name`. Note that no trailing `;` is
    /// printed.
    ///
    /// This can be used to declare local and global variables as well as struct
    /// fields.
    pub fn declare_variable<W: WriteStr>(
        &self,
        name: impl AsRef<str>,
        w: &mut W,
    ) -> io::Result<()> {
        self.0.declare_variable(name.as_ref(), w)
    }
}

/// Reference to an incomplete C type.
#[derive(Debug, Copy, Clone)]
pub struct IncompleteTypeRef<'a>(pub &'a Type<'a>);

impl<'a> AsType<'a> for IncompleteTypeRef<'a> {
    fn as_type(&self) -> &'a Type<'a> {
        self.0
    }
}
