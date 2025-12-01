//! Definitions of AST types for precondition representation and translation.

mod block;
mod expr;
mod op;
mod path;
mod types;

pub use block::*;
pub use expr::*;
pub use op::*;
pub use path::*;
pub use types::*;

/// Re-export Verus function signature type.
pub type Signature = verus_syn::Signature;
/// Re-export Verus generics type.
pub type Generics = verus_syn::Generics;

/// A function's name, signature, and its precondition expressions.
#[derive(Clone)]
pub struct FunctionPrecond {
    /// Fully qualified function name.
    pub name: Path,
    /// Function signature.
    pub signature: Signature,
    /// Precondition expressions.
    pub requires: Vec<Expr>,
}

/// A method's impl type, signature, and its precondition expressions.
#[derive(Clone)]
pub struct MethodPrecond {
    /// Generics
    pub generics: Generics,
    /// Impl type.
    pub impl_type: Type,
    /// Method signature.
    pub signature: Signature,
    /// Precondition expressions.
    pub requires: Vec<Expr>,
}

impl MethodPrecond {
    /// Get the fully qualified method name.
    pub fn name(&self) -> Path {
        self.impl_type
            .as_path()
            .join(self.signature.ident.to_string())
    }
}

/// A free-standing spec function.
#[derive(Clone)]
pub struct SpecFunction {
    /// Function name.
    pub name: Path,
    /// Function signature.
    pub signature: Signature,
    /// Function body.
    pub body: Block,
}

/// A spec function within an impl block.
#[derive(Clone)]
pub struct SpecMethod {
    /// Generics
    pub generics: Generics,
    /// Impl type.
    pub impl_type: Type,
    /// Method signature.
    pub signature: Signature,
    /// Method body.
    pub body: Block,
}

impl SpecMethod {
    /// Get the fully qualified method name.
    pub fn name(&self) -> Path {
        self.impl_type
            .as_path()
            .join(self.signature.ident.to_string())
    }
}
