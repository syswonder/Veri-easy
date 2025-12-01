//! Collects all concrete instantiations of generic types in the Verus AST.
//!
//! Only explicit instantiations (like `type FooBar = Foo<Bar>`) are collected. The alias
//! type (`FooBar`) should not contain any generics.

use crate::defs::{InstantiatedType, Path, Type};
use syn::{ItemType, visit::Visit};

/// Visitor that collects instantiations of generic types.
pub struct TypeCollector {
    /// Collected type aliases.
    types: Vec<ItemType>,
}

impl TypeCollector {
    /// Create a new TypeCollector.
    pub fn new() -> Self {
        TypeCollector { types: Vec::new() }
    }

    /// Collect instantiated types from the given syntax tree.
    pub fn collect(mut self, syntax: &syn::File) -> Vec<InstantiatedType> {
        self.visit_file(syntax);

        let mut instantiated_types = Vec::new();
        for item in self.types {
            let path = Path(vec![item.ident.to_string()]);
            if let Ok(concrete_type) = Type::try_from(*item.ty) {
                if let Type::Generic(_) = &concrete_type {
                    instantiated_types.push(InstantiatedType {
                        alias: path,
                        concrete: concrete_type,
                    });
                }
            }
        }
        instantiated_types
    }
}

impl<'ast> Visit<'ast> for TypeCollector {
    fn visit_item_type(&mut self, i: &'ast ItemType) {
        self.types.push(i.clone());
    }
}
