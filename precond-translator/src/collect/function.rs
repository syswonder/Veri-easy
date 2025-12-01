//! Collect Verus spec functions.
use super::path::PathResolver;
use crate::ast::{Path, Type};
use verus_syn::{
    Block, FnMode, Generics, ImplItemFn, ItemFn, ItemImpl, ItemMod, ItemUse, Signature,
    visit::{self, Visit},
};

/// A free-standing spec function.
struct SpecFunction {
    /// Function name.
    name: Path,
    /// Function signature.
    signature: Signature,
    /// Function body.
    body: Block,
}

/// A spec function within an impl block.
struct SpecMethod {
    /// Impl generics.
    generics: Generics,
    /// Impl type name.
    impl_type: Type,
    /// Method signature.
    signature: Signature,
    /// Method body.
    body: Block,
}

/// Visit that visits Verus AST and extracts spec functions.
///
/// Spec functions may be defined as free-standing functions or impl methods.
pub struct SpecFunctionCollector<'ast> {
    /// Collected spec functions.
    spec_functions: Vec<SpecFunction>,
    /// Collected spec methods.
    spec_methods: Vec<SpecMethod>,
    /// Store currently visited impl block
    impl_block: Option<&'ast ItemImpl>,
    /// Path resolver.
    resolver: PathResolver,
}

impl<'ast> SpecFunctionCollector<'ast> {
    /// Create a new SpecFunctionCollector.
    pub fn new() -> Self {
        SpecFunctionCollector {
            spec_functions: Vec::new(),
            spec_methods: Vec::new(),
            impl_block: None,
            resolver: PathResolver::new(),
        }
    }
    /// Collect spec functions from the given Verus syntax tree.
    pub fn collect(
        mut self,
        syntax: &'ast verus_syn::File,
    ) -> (Vec<crate::ast::SpecFunction>, Vec<crate::ast::SpecMethod>) {
        verus_syn::visit::visit_file(&mut self, syntax);
        let mut spec_functions = Vec::new();
        for spec_function in self.spec_functions {
            if let Ok(body) = crate::ast::Block::try_from(spec_function.body) {
                spec_functions.push(crate::ast::SpecFunction {
                    name: spec_function.name,
                    signature: spec_function.signature,
                    body,
                });
            }
        }
        let mut spec_methods = Vec::new();
        for spec_method in self.spec_methods {
            if let Ok(body) = crate::ast::Block::try_from(spec_method.body) {
                spec_methods.push(crate::ast::SpecMethod {
                    generics: spec_method.generics,
                    impl_type: spec_method.impl_type,
                    signature: spec_method.signature,
                    body,
                });
            }
        }
        (spec_functions, spec_methods)
    }
}

impl<'ast> Visit<'ast> for SpecFunctionCollector<'ast> {
    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        self.resolver.enter_module(i);
        visit::visit_item_mod(self, i);
        self.resolver.exit_module();
    }

    fn visit_item_use(&mut self, i: &'ast ItemUse) {
        self.resolver.parse_use_tree(&i.tree, Path::empty());
    }

    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if !i.sig.generics.params.is_empty() {
            return;
        } // Skip generic functions
        // Only collect spec functions
        if !matches!(i.sig.mode, FnMode::Spec(_)) {
            return;
        }
        let name = self.resolver.concat_module(&i.sig.ident.to_string());
        self.spec_functions.push(SpecFunction {
            name,
            signature: i.sig.clone(),
            body: *i.block.clone(),
        });
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        self.impl_block = Some(i);
        visit::visit_item_impl(self, i);
        self.impl_block = None;
    }

    fn visit_impl_item_fn(&mut self, i: &'ast ImplItemFn) {
        if !i.sig.generics.params.is_empty() {
            return;
        } // Skip generic functions
        // Only collect spec functions
        if !matches!(i.sig.mode, FnMode::Spec(_)) {
            return;
        }
        let impl_block = self.impl_block.cloned().unwrap();
        if let Ok(mut self_ty) = Type::try_from(*impl_block.self_ty) {
            match &mut self_ty {
                Type::Generic(g) => g.path = self.resolver.resolve_path(&g.path),
                Type::Precise(p) => p.0 = self.resolver.resolve_path(&p.0),
            }
            self.spec_methods.push(SpecMethod {
                impl_type: self_ty,
                generics: impl_block.generics,
                signature: i.sig.clone(),
                body: i.block.clone(),
            });
        }
    }
}
