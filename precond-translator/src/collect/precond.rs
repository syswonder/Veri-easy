//! Collect Verus function preconditions.
use super::path::PathResolver;
use crate::ast::Path;
use verus_syn::{
    FnMode, Generics, Ident, ImplItemFn, ItemFn, ItemImpl, ItemMod, ItemTrait, ItemUse, Requires,
    Signature, SignatureSpec, TraitItemFn, Type,
    visit::{self, Visit},
};

/// Precondition defined in trait.
struct TraitPrecond {
    /// Trait name.
    trait_name: Path,
    /// Function signature.
    signature: Signature,
    /// Preconditions.
    requires: Requires,
}

/// Precondition defined in free-standing function.
struct FunctionPrecond {
    /// Function name.
    func_name: Path,
    /// Function signature.
    signature: Signature,
    /// Preconditions.
    requires: Requires,
}

/// Precondition defined in impl method.
struct MethodPrecond {
    /// Generics
    generics: Generics,
    /// Impl type.
    impl_type: Type,
    /// Function signature.
    signature: Signature,
    /// Preconditions.
    requires: Requires,
}

/// Visitor that visits Verus AST and extracts preconditions of executable functions.
///
/// Precondtion may be defined in trait or directly in function/method.
///
/// - For trait methods, we collect the trait name, the function signature, and the preconditions.
/// - For free-standing functions and impl methods, we collect the function name, the function signature and the preconditions.
/// - Generic functions are skipped for simplicity.
///
/// If a trait is implemented for a struct, the trait method preconditions are transferred to the
/// corresponding impl function preconditions during later processing.
pub struct PrecondCollector<'ast> {
    /// Preconditions defined in trait
    trait_preconds: Vec<TraitPrecond>,
    /// Preconditions defined in free-standing functions
    func_preconds: Vec<FunctionPrecond>,
    /// Preconditions defined in impl methods
    method_preconds: Vec<MethodPrecond>,
    /// Store trait-impl info: (trait name, generics, type)
    trait_impls: Vec<(Path, Generics, Type)>,
    /// Store currently visited trait identifier
    trait_: Option<&'ast Ident>,
    /// Store currently visited function signature
    function: Option<&'ast Signature>,
    /// Store currently visited impl block
    impl_block: Option<&'ast ItemImpl>,
    /// Path resolver.
    resolver: PathResolver,
}

impl<'ast> PrecondCollector<'ast> {
    /// Create a new PreconditionCollector.
    pub fn new() -> Self {
        PrecondCollector {
            trait_preconds: Vec::new(),
            func_preconds: Vec::new(),
            method_preconds: Vec::new(),
            trait_impls: Vec::new(),
            trait_: None,
            function: None,
            impl_block: None,
            resolver: PathResolver::new(),
        }
    }

    /// Collect preconditions from the given Verus syntax tree, and transform into our AST form.
    pub fn collect(
        mut self,
        syntax: &'ast verus_syn::File,
    ) -> (
        Vec<crate::ast::FunctionPrecond>,
        Vec<crate::ast::MethodPrecond>,
    ) {
        self.visit_file(syntax);

        let mut function_preconds = Vec::new();
        // Collect free-standing function preconditions
        for precondition in self.func_preconds {
            let mut req_exprs = Vec::new();
            for expr in &precondition.requires.exprs.exprs {
                if let Ok(req_expr) = expr.clone().try_into() {
                    req_exprs.push(req_expr);
                }
            }
            function_preconds.push(crate::ast::FunctionPrecond {
                name: precondition.func_name.clone(),
                requires: req_exprs,
                signature: precondition.signature.clone(),
            });
        }

        let mut method_preconds = Vec::new();
        // Collect impl method preconditions
        for precondition in self.method_preconds {
            let mut req_exprs = Vec::new();
            for expr in &precondition.requires.exprs.exprs {
                if let Ok(req_expr) = expr.clone().try_into() {
                    req_exprs.push(req_expr);
                }
            }
            if let Ok(impl_type) = crate::ast::Type::try_from(precondition.impl_type) {
                method_preconds.push(crate::ast::MethodPrecond {
                    generics: precondition.generics,
                    impl_type,
                    signature: precondition.signature,
                    requires: req_exprs,
                });
            }
        }
        // Collect trait-implemented method preconditions
        for precondition in self.trait_preconds {
            let impl_types: Vec<(&Generics, &Type)> = self
                .trait_impls
                .iter()
                .filter_map(|(tr, gr, ty)| {
                    if *tr == precondition.trait_name {
                        Some((gr, ty))
                    } else {
                        None
                    }
                })
                .collect();
            for (generics, impl_type) in impl_types {
                let mut req_exprs = Vec::new();
                for expr in &precondition.requires.exprs.exprs {
                    if let Ok(req_expr) = expr.clone().try_into() {
                        req_exprs.push(req_expr);
                    }
                }
                if let Ok(impl_type) = crate::ast::Type::try_from(impl_type.clone()) {
                    method_preconds.push(crate::ast::MethodPrecond {
                        generics: generics.clone(),
                        impl_type,
                        signature: precondition.signature.clone(),
                        requires: req_exprs,
                    });
                }
            }
        }

        (function_preconds, method_preconds)
    }
}

impl<'ast> Visit<'ast> for PrecondCollector<'ast> {
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
        self.function = Some(&i.sig);
        visit::visit_item_fn(self, i);
        self.function = None;
    }

    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        self.trait_ = Some(&i.ident);
        visit::visit_item_trait(self, i);
        self.trait_ = None;
    }

    fn visit_trait_item_fn(&mut self, i: &'ast TraitItemFn) {
        self.function = Some(&i.sig);
        visit::visit_trait_item_fn(self, i);
        self.function = None;
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        if let Some((_, path, _)) = &i.trait_ {
            // Record trait-impl mapping
            let trait_name = self.resolver.resolve_path(&path.clone().into());
            self.trait_impls
                .push((trait_name, i.generics.clone(), *i.self_ty.clone()));
        }
        self.impl_block = Some(i);
        visit::visit_item_impl(self, i);
        self.impl_block = None;
    }

    fn visit_impl_item_fn(&mut self, i: &'ast ImplItemFn) {
        if !i.sig.generics.params.is_empty() {
            return;
        } // Skip generic functions
        self.function = Some(&i.sig);
        visit::visit_impl_item_fn(self, i);
        self.function = None;
    }

    fn visit_signature_spec(&mut self, i: &'ast SignatureSpec) {
        if self.function.is_none() {
            return;
        }
        let function = self.function.unwrap();
        // Only collect preconditions for executable functions
        if !matches!(function.mode, FnMode::Exec(_)) && !matches!(function.mode, FnMode::Default) {
            return;
        }
        if i.requires.is_none() {
            return;
        }
        let requires = i.requires.clone().unwrap();

        // Collect precondition
        if let Some(trait_ident) = self.trait_ {
            // Trait method precondition
            let trait_name = self.resolver.concat_module(&trait_ident.to_string());
            self.trait_preconds.push(TraitPrecond {
                trait_name,
                signature: function.clone(),
                requires,
            });
            return;
        }
        if let Some(impl_block) = self.impl_block {
            // Impl method precondition
            self.method_preconds.push(MethodPrecond {
                impl_type: (*impl_block.self_ty).clone(),
                generics: impl_block.generics.clone(),
                signature: function.clone(),
                requires,
            });
            return;
        }
        // Free-standing function precondition
        let func_name = self.resolver.concat_module(&function.ident.to_string());
        self.func_preconds.push(FunctionPrecond {
            func_name,
            signature: function.clone(),
            requires,
        });
    }
}
