//! Collect functions from two programs.

mod function;
mod path;
mod precond;
mod symbol;
mod types;

pub use function::FunctionCollector;
pub use path::PathResolver;
pub use precond::collect_preconds;
pub use symbol::SymbolCollector;
pub use types::TypeCollector;
