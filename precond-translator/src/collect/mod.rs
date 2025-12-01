//! Collect Verus function preconditions into AST form defined in `ast` module.

mod function;
mod path;
mod precond;

pub use function::SpecFunctionCollector;
pub use precond::PrecondCollector;
