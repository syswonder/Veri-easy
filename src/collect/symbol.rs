//! Collect import symbols from a Rust program.
use syn::{ItemTrait, visit::Visit};

use crate::{collect::path::ModuleStack, defs::Path};

/// Visitor that collects symbols. For now, only traits are collected.
pub struct SymbolCollector {
    /// Collected traits.
    traits: Vec<Path>,
    /// Module stack.
    module: ModuleStack,
}

impl SymbolCollector {
    /// Create a new symbol collector.
    pub fn new() -> Self {
        Self {
            traits: Vec::new(),
            module: ModuleStack::new(),
        }
    }
    /// Collect symbols from the syntax tree.
    pub fn collect(mut self, syntax: &syn::File) -> Vec<Path> {
        self.visit_file(syntax);
        self.traits
    }
}

impl<'ast> Visit<'ast> for SymbolCollector {
    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        let trait_path = self.module.concat(&i.ident.to_string());
        self.traits.push(trait_path);
    }
}
