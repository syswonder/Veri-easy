//! Collect functions from a Rust program.

use crate::{
    collect::path::ModuleStack,
    defs::{Path, Type},
};
use syn::{
    Block, File, ImplItemFn, ItemFn, ItemImpl, ItemMod, Signature,
    visit::{self, Visit},
};

/// Represent a function parsed from source code.
struct Function {
    /// Fully qualified name of the function.
    name: Path,
    /// Function signature.
    signature: Signature,
    /// The impl type if it's an impl method.
    impl_type: Option<Type>,
    /// Function body.
    body: Block,
}

/// Visitor that collects free functions and impl methods.
pub struct FunctionCollector<'ast> {
    /// Collected functions.
    functions: Vec<Function>,
    /// Currently visited impl block.
    impl_block: Option<&'ast ItemImpl>,
    /// Module stack.
    module: ModuleStack,
}

impl<'ast> FunctionCollector<'ast> {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            impl_block: None,
            module: ModuleStack::new(),
        }
    }
    pub fn collect(mut self, syntax: &'ast File) -> Vec<crate::defs::Function> {
        self.visit_file(syntax);

        let mut functions = Vec::new();
        for func in self.functions {
            let body = func.body;
            functions.push(crate::defs::Function::new(
                crate::defs::FunctionMetadata::new(
                    func.name,
                    crate::defs::Signature(func.signature),
                    func.impl_type,
                ),
                quote::quote! { #body }.to_string(),
            ));
        }
        functions
    }
}

impl<'ast> Visit<'ast> for FunctionCollector<'ast> {
    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        self.module.push(&i.ident.to_string());
        visit::visit_item_mod(self, i);
        self.module.pop();
    }

    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if !i.sig.generics.params.is_empty() {
            return;
        } // Skip generic functions
        if i.attrs.iter().any(|attr| attr.path().is_ident("ignore")) {
            return;
        } // Skip functions marked with #[ignore]

        let name = self.module.concat(&i.sig.ident.to_string());
        self.functions.push(Function {
            name,
            signature: i.sig.clone(),
            impl_type: None,
            body: (*i.block).clone(),
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
        if i.attrs.iter().any(|attr| attr.path().is_ident("ignore")) {
            return;
        } // Skip functions marked with #[ignore]

        let impl_block = self.impl_block.cloned().unwrap();
        if let Ok(self_ty) = Type::try_from(*impl_block.self_ty) {
            // self_ty is already resolved by `PathResolver`
            let name = self_ty.to_path().join(i.sig.ident.to_string());
            self.functions.push(Function {
                name,
                impl_type: Some(self_ty),
                signature: i.sig.clone(),
                body: i.block.clone(),
            });
        }
    }
}
