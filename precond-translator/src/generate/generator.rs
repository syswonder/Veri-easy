//! Generate excutable precondition checking functions and spec functions/methods.

use super::visitors::*;
use crate::ast::*;
use crate::visit::{Visit, VisitMut};
use proc_macro2::TokenStream;
use quote::quote;
use std::str::FromStr;

/// Generate excutable precondition checking functions and spec functions/methods.
pub struct CodeGenerator {
    /// Collected spec functions.
    spec_functions: Vec<SpecFunction>,
    /// Collected spec methods.
    spec_methods: Vec<SpecMethod>,
    /// Collected preconditions of free-standing functions.
    function_preconds: Vec<FunctionPrecond>,
    /// Collected preconditions of methods.
    method_preconds: Vec<MethodPrecond>,
}

impl CodeGenerator {
    /// Create a new code generator.
    pub fn new(
        spec_fns: Vec<SpecFunction>,
        spec_methods: Vec<SpecMethod>,
        function_preconds: Vec<FunctionPrecond>,
        method_preconds: Vec<MethodPrecond>,
    ) -> Self {
        let mut generstor = CodeGenerator {
            spec_functions: spec_fns,
            spec_methods,
            function_preconds,
            method_preconds,
        };
        generstor.preprocess();
        generstor
    }

    /// Generate all code.
    pub fn generate_all(&self) -> TokenStream {
        let mut tokens = Vec::new();
        for spec_function in &self.spec_functions {
            tokens.push(self.generate_spec_function(spec_function));
        }
        for spec_method in &self.spec_methods {
            tokens.push(self.generate_spec_method(spec_method));
        }
        for precond in &self.function_preconds {
            tokens.push(self.generate_function_precond(precond));
        }
        for precond in &self.method_preconds {
            tokens.push(self.generate_method_precond(precond));
        }
        quote! {
            #(#tokens)*
        }
    }

    /// Get all precondition checking function for free-standing functions.
    pub fn get_function_preconds(&self) -> Vec<String> {
        self.function_preconds
            .iter()
            .map(|f| f.name.to_string())
            .collect()
    }

    /// Get all precondition checking function for methods.
    pub fn get_method_preconds(&self) -> Vec<String> {
        self.method_preconds
            .iter()
            .map(|f| f.name().to_string())
            .collect()
    }

    /// Preprocess for code generation.
    ///
    /// - Remove "old" function calls.
    /// - Remove non-generatable spec functions/methods from allowed list.
    /// - Remove non-generatable require expressions.
    fn preprocess(&mut self) {
        // Remove "old" in spec functions and methods.
        for precond in &mut self.function_preconds {
            for req in &mut precond.requires {
                let mut remover = RemoveOld;
                remover.visit_expr_mut(req);
            }
        }
        // Remove "old" in method preconditions.
        for precond in &mut self.method_preconds {
            for req in &mut precond.requires {
                let mut remover = RemoveOld;
                remover.visit_expr_mut(req);
            }
        }

        let allowed_fns = Self::calculate_allowed_fns(&self.spec_functions, &self.spec_methods);
        // Remove non-generatable spec functions/methods from allowed list.
        self.spec_functions
            .retain(|f| Self::is_spec_fn_generatable(&allowed_fns, &f.body, None));
        self.spec_methods
            .retain(|m| Self::is_spec_fn_generatable(&allowed_fns, &m.body, Some(&m.impl_type)));

        // Remove non-generatable require expressions.
        for precond in &mut self.function_preconds {
            precond
                .requires
                .retain(|req| Self::is_require_generatable(&allowed_fns, req, None));
        }
        for precond in &mut self.method_preconds {
            precond.requires.retain(|req| {
                Self::is_require_generatable(&allowed_fns, req, Some(&precond.impl_type))
            });
        }

        // Replace "spec_foo" with "foo" in function preconditions.
        for precond in &mut self.function_preconds {
            for req in &mut precond.requires {
                let mut remover = RemoveSpecPrefix;
                remover.visit_expr_mut(req);
            }
        }
        // Replace "spec_foo" with "foo" in method preconditions.
        for precond in &mut self.method_preconds {
            for req in &mut precond.requires {
                let mut remover = RemoveSpecPrefix;
                remover.visit_expr_mut(req);
            }
        }
    }

    /// Generate exec version of a spec function.
    fn generate_spec_function(&self, spec_fn: &SpecFunction) -> TokenStream {
        let fn_name = spec_fn.name.to_ident();
        let fn_name_ts = TokenStream::from_str(&fn_name).unwrap();
        let inputs = &spec_fn.signature.inputs;
        let output = match &spec_fn.signature.output {
            verus_syn::ReturnType::Default => quote! {},
            verus_syn::ReturnType::Type(_, _, _, ty) => quote! { -> #ty },
        };

        let mut generator = AstToCode::new();
        generator.visit_block(&spec_fn.body);
        let body_ts = generator.get_code();

        quote! {
            pub fn #fn_name_ts(#inputs) #output #body_ts
        }
    }

    /// Generate exec version of a spec method.
    fn generate_spec_method(&self, spec_method: &SpecMethod) -> TokenStream {
        let generics = &spec_method.generics;
        let impl_type =
            TokenStream::from_str(&spec_method.impl_type.as_path().to_string()).unwrap();
        let fn_name = spec_method.signature.ident.to_string();
        let fn_name_ts = TokenStream::from_str(&fn_name).unwrap();
        let inputs = &spec_method.signature.inputs;
        let output = match &spec_method.signature.output {
            verus_syn::ReturnType::Default => quote! {},
            verus_syn::ReturnType::Type(_, _, _, ty) => quote! { -> #ty },
        };

        let mut generator = AstToCode::new();
        generator.visit_block(&spec_method.body);
        let body_ts = generator.get_code();

        quote! {
            impl #generics #impl_type {
                pub fn #fn_name_ts(#inputs) #output #body_ts
            }
        }
    }

    /// Generate checking function for a precondition of a free-standing function.
    fn generate_function_precond(&self, precond: &FunctionPrecond) -> TokenStream {
        let fn_name = "verieasy_pre_".to_owned() + &precond.name.to_ident();
        let fn_name_ts = TokenStream::from_str(&fn_name).unwrap();
        let inputs = precond.signature.inputs.clone();

        let mut requires = Vec::new();
        for req in &precond.requires {
            // Generate code.
            let mut generator = AstToCode::new();
            generator.visit_expr(req);
            requires.push(generator.get_code());
        }

        quote! {
            pub fn #fn_name_ts(#inputs) -> bool {
                #(if !(#requires) { return false; })*
                true
            }
        }
    }

    /// Generate checking function for a precondition of a method.
    fn generate_method_precond(&self, precond: &MethodPrecond) -> TokenStream {
        let generics = &precond.generics;
        let impl_type = TokenStream::from_str(&precond.impl_type.as_path().to_string()).unwrap();
        let fn_name = "verieasy_pre_".to_owned() + &precond.signature.ident.to_string();
        let fn_name_ts = TokenStream::from_str(&fn_name).unwrap();
        let inputs = precond.signature.inputs.clone();

        let mut requires = Vec::new();
        for req in &precond.requires {
            // Generate code.
            let mut generator = AstToCode::new();
            generator.visit_expr(req);
            requires.push(generator.get_code());
        }

        quote! {
            impl #generics #impl_type {
                pub fn #fn_name_ts(#inputs) -> bool {
                    #(if !(#requires) { return false; })*
                    true
                }
           }
        }
    }

    /// Check if a require expression is generatable.
    fn is_require_generatable(allowed_fns: &[Path], req: &Expr, self_ty: Option<&Type>) -> bool {
        let mut checker = CheckFnCall::new(allowed_fns, self_ty);
        checker.visit_expr(req);
        !checker.aborted
    }

    /// Check if a spec function or method is generatable.
    fn is_spec_fn_generatable(allowed_fns: &[Path], body: &Block, self_ty: Option<&Type>) -> bool {
        let mut checker = CheckFnCall::new(allowed_fns, self_ty);
        checker.visit_block(body);
        !checker.aborted
    }

    /// Calculate the allowed functions and methods for generating.
    fn calculate_allowed_fns(spec_fns: &[SpecFunction], spec_methods: &[SpecMethod]) -> Vec<Path> {
        let mut allowed_fns = spec_fns
            .iter()
            .map(|f| f.name.clone())
            .chain(spec_methods.iter().map(|m| m.name()))
            .collect::<Vec<Path>>();

        let mut len = allowed_fns.len();
        // Iterate until no more functions can be removed.
        loop {
            for spec_fn in spec_fns {
                if !Self::is_spec_fn_generatable(&allowed_fns, &spec_fn.body, None) {
                    allowed_fns.retain(|p| *p != spec_fn.name);
                }
            }
            for method in spec_methods {
                if !Self::is_spec_fn_generatable(
                    &allowed_fns,
                    &method.body,
                    Some(&method.impl_type),
                ) {
                    allowed_fns.retain(|p| *p != method.name());
                }
            }
            if allowed_fns.len() == len {
                break;
            }
            len = allowed_fns.len();
        }
        allowed_fns
    }
}
