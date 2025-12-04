//! Harness generator used by various steps (Kani, PBT, DFT).
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;

use crate::{
    check::Checker,
    defs::{CommonFunction, Path, Precondition, Type},
    log,
};

/// Structure that stores functions into 4 different categories:
///
/// - Free-standing functions (without `self` receiver)
/// - methods (with `self` receiver)
/// - constructors (functions that has name `verieasy_new` inside an `impl` block)
/// - state getters (functions that has name `verieasy_get` inside an `impl` block)
#[derive(Debug)]
pub struct FunctionCollection {
    /// Free-standing functions.
    pub functions: Vec<CommonFunction>,
    /// Methods.
    pub methods: Vec<CommonFunction>,
    /// Constructors mapped by their type.
    pub constructors: BTreeMap<Type, CommonFunction>,
    /// State getters mapped by their type.
    pub getters: BTreeMap<Type, CommonFunction>,
    /// Preconditions
    pub preconditions: Vec<Precondition>,
}

impl FunctionCollection {
    /// Classify functions into free-standing functions, methods.
    ///
    /// Construct map for constructors and getters.
    pub fn new(
        functions: Vec<CommonFunction>,
        constructors: Vec<CommonFunction>,
        getters: Vec<CommonFunction>,
        preconditions: Vec<Precondition>,
    ) -> Self {
        let mut res = Self {
            functions: Vec::new(),
            methods: Vec::new(),
            constructors: BTreeMap::new(),
            getters: BTreeMap::new(),
            preconditions,
        };
        for func in functions {
            if let Some(_) = &func.metadata.impl_type {
                if func
                    .metadata
                    .signature
                    .0
                    .inputs
                    .iter()
                    .any(|arg| matches!(arg, syn::FnArg::Receiver(_)))
                {
                    // Has `self` receiver, consider it as a method.
                    res.methods.push(func);
                } else {
                    // No `self` receiver, consider it as a free-standing function.
                    res.functions.push(func);
                }
            } else {
                // Function outside of impl block is a free-standing function.
                res.functions.push(func);
            }
        }
        for constructor in constructors {
            if let Some(impl_type) = &constructor.metadata.impl_type {
                res.constructors.insert(impl_type.clone(), constructor);
            }
        }
        for getter in getters {
            if let Some(impl_type) = &getter.metadata.impl_type {
                res.getters.insert(impl_type.clone(), getter);
            }
        }
        res
    }

    /// Get the precondition for the given function.
    pub fn get_precondition(&self, func: &CommonFunction) -> Option<&Precondition> {
        self.preconditions
            .iter()
            .find(|pre| pre.name == func.metadata.name)
    }

    /// If `methods` doesn't have a method of type `T`, then its constructor and getter asre unused.
    ///
    /// This function removes those constructors and getters.
    pub fn remove_unused_constructors_and_getters(&mut self) {
        let mut unused_types = Vec::new();
        for (type_, _) in &self.constructors {
            if !self
                .methods
                .iter()
                .any(|method| method.metadata.impl_type.as_ref() == Some(type_))
            {
                unused_types.push(type_.clone());
            }
        }
        for type_ in &unused_types {
            log!(
                Verbose,
                Warning,
                "Type `{:?}` doesn't have any methods, remove its constructor and getter.",
                type_.to_path()
            );
            self.constructors.remove(type_);
            self.getters.remove(type_);
        }
    }

    /// If `methods` has a method of type `T`, but `constructors` doesn't have a constructor of type `T`.
    ///
    /// This function removes those methods.
    pub fn remove_methods_without_constructors(&mut self) {
        let mut no_constructor_types = Vec::new();
        for method in &self.methods {
            if !self.constructors.contains_key(method.impl_type())
                && !no_constructor_types.iter().any(|t| t == method.impl_type())
            {
                no_constructor_types.push(method.impl_type().clone());
            }
        }
        for type_ in &no_constructor_types {
            log!(
                Normal,
                Warning,
                "Type `{:?}` doesn't have a constructor, skip all its methods.",
                type_.to_path()
            );
            self.methods
                .retain(|m| m.metadata.impl_type.as_ref() != Some(type_));
        }
    }
}

/// Generic harness generator using a backend.
pub struct HarnessGenerator<B: HarnessBackend> {
    /// Functions used to generate the harness
    pub collection: FunctionCollection,
    /// Imports from mod1
    pub mod1_imports: Vec<Path>,
    /// Imports from mod2
    pub mod2_imports: Vec<Path>,
    /// Backend marker
    pub backend: B,
}

impl<B: HarnessBackend> HarnessGenerator<B> {
    /// Create a new harness generator for the given functions.
    pub fn new(checker: &Checker, backend: B) -> Self {
        let mut collection = FunctionCollection::new(
            checker.under_checking_funcs.clone(),
            checker.constructors.clone(),
            checker.getters.clone(),
            checker.preconditions.clone(),
        );
        collection.remove_unused_constructors_and_getters();
        collection.remove_methods_without_constructors();
        Self {
            collection,
            mod1_imports: checker.src1.symbols.clone(),
            mod2_imports: checker.src2.symbols.clone(),
            backend,
        }
    }

    /// Generate argument struct `ArgsFoo` for function `foo`; backend supplies the derive/attrs.
    fn generate_arg_struct(&self, func: &CommonFunction) -> TokenStream {
        let struct_name = format_ident!("Args{}", func.metadata.name.to_ident());
        let mut fields = Vec::<TokenStream>::new();
        for arg in &func.metadata.signature.0.inputs {
            if matches!(arg, syn::FnArg::Typed(_)) {
                fields.push(quote! { #arg });
            }
        }
        let attrs = self.backend.arg_struct_attrs();
        quote! {
            #attrs
            pub struct #struct_name {
                #(pub #fields),*
            }
        }
    }

    /// Generate all argument structs for functions, methods, and constructors.
    fn generate_all_arg_structs(&self) -> Vec<TokenStream> {
        let mut func_structs = self
            .collection
            .functions
            .iter()
            .map(|f| self.generate_arg_struct(f))
            .collect::<Vec<_>>();

        let mut method_structs = Vec::<TokenStream>::new();
        let mut used_constructors = Vec::<&CommonFunction>::new();
        for method in &self.collection.methods {
            let constructor = self
                .collection
                .constructors
                .get(method.impl_type())
                .unwrap();
            method_structs.push(self.generate_arg_struct(method));
            if !used_constructors
                .iter()
                .any(|c| c.metadata.name == constructor.metadata.name)
            {
                used_constructors.push(&constructor);
            }
        }

        let constructor_structs = used_constructors
            .iter()
            .map(|c| self.generate_arg_struct(c))
            .collect::<Vec<_>>();

        func_structs.extend(constructor_structs);
        func_structs.extend(method_structs);
        func_structs
    }

    /// Generate a harness function for comparing two free-standing functions.
    fn generate_harness_for_function(&self, func: &CommonFunction) -> TokenStream {
        let precondition = self.collection.get_precondition(func);

        let mut function_args = Vec::<TokenStream>::new();
        for arg in &func.metadata.signature.0.inputs {
            if let syn::FnArg::Typed(pat_type) = arg {
                let arg_name = match &*pat_type.pat {
                    syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
                    _ => "arg".to_string(),
                };
                let ident = format_ident!("{}", arg_name);
                function_args.push(quote! { #ident.clone() });
            }
        }
        self.backend
            .make_harness_for_function(func, &function_args, precondition)
    }

    /// Generate a harness function for comparing two methods.
    fn generate_harness_for_method(&self, method: &CommonFunction) -> TokenStream {
        let constructor = self
            .collection
            .constructors
            .get(method.impl_type())
            .unwrap();
        // getter may be absent
        let getter = self.collection.getters.get(method.impl_type());
        let precondition = self.collection.get_precondition(method);

        // collect constructor args
        let mut constructor_args = Vec::new();
        for arg in &constructor.metadata.signature.0.inputs {
            if let syn::FnArg::Typed(pat_type) = arg {
                let name = match &*pat_type.pat {
                    syn::Pat::Ident(pi) => pi.ident.to_string(),
                    _ => "arg".into(),
                };
                let ident = format_ident!("{}", name);
                constructor_args.push(quote! { #ident.clone() });
            }
        }

        // method args and receiver info
        let mut method_args = Vec::new();
        let mut receiver_mut = None;
        let mut receiver_ref = None;
        for arg in &method.metadata.signature.0.inputs {
            match arg {
                syn::FnArg::Receiver(rec) => {
                    receiver_mut = rec.mutability.clone();
                    receiver_ref = rec.reference.clone();
                }
                syn::FnArg::Typed(pat) => {
                    let name = match &*pat.pat {
                        syn::Pat::Ident(pi) => pi.ident.to_string(),
                        _ => "arg".into(),
                    };
                    let ident = format_ident!("{}", name);
                    method_args.push(quote! { #ident.clone() });
                }
            }
        }
        let receiver_prefix = {
            let reference = receiver_ref.map(|(amp, _)| amp);
            let mut_tok = receiver_mut;
            // We will call backend with something like `#reference #mut` as the receiver prefix.
            quote! { #reference #mut_tok }
        };

        self.backend.make_harness_for_method(
            method,
            constructor,
            getter,
            &method_args,
            &constructor_args,
            receiver_prefix,
            precondition,
        )
    }

    /// Generate trait imports (`use` statements) for the harness file.
    fn generate_imports(&self) -> Vec<TokenStream> {
        let mod1_import_stmts = self.mod1_imports.iter().map(|path| {
            let ident = format_ident!("Mod1{}", path.0.last().unwrap());
            quote! {
                use mod1::#path as #ident;
            }
        });
        let mod2_import_stmts = self.mod2_imports.iter().map(|path| {
            let ident = format_ident!("Mod2{}", path.0.last().unwrap());
            quote! {
                use mod2::#path as #ident;
            }
        });
        mod1_import_stmts.chain(mod2_import_stmts).collect()
    }

    /// Generate the complete harness file as a TokenStream.
    pub fn generate_harness(&self) -> TokenStream {
        let imports = self.generate_imports();
        let arg_structs = self.generate_all_arg_structs();
        let functions = self
            .collection
            .functions
            .iter()
            .map(|func| self.generate_harness_for_function(func))
            .collect::<Vec<_>>();
        let methods = self
            .collection
            .methods
            .iter()
            .map(|method| self.generate_harness_for_method(method))
            .collect::<Vec<_>>();
        let additional = self.backend.additional_code(&self.collection);

        self.backend
            .finalize(imports, arg_structs, functions, methods, additional)
    }
}

/// The trait capturing differences between different check/test harness backends.
pub trait HarnessBackend {
    /// Attributes / derives to put on generated `Args*` structs.
    fn arg_struct_attrs(&self) -> TokenStream;

    /// Build the test function TokenStream for a free-standing function.
    fn make_harness_for_function(
        &self,
        function: &CommonFunction,
        function_args: &[TokenStream],
        precondition: Option<&Precondition>,
    ) -> TokenStream;

    /// Build the test function TokenStream for a method.
    fn make_harness_for_method(
        &self,
        method: &CommonFunction,
        constructor: &CommonFunction,
        getter: Option<&CommonFunction>,
        method_args: &[TokenStream],
        constructor_args: &[TokenStream],
        receiver_prefix: TokenStream,
        precondition: Option<&Precondition>,
    ) -> TokenStream;

    /// Other additional code pieces needed can be added as associated functions here.
    fn additional_code(&self, _classifier: &FunctionCollection) -> TokenStream {
        quote! {}
    }

    /// Final wrapper given all pieces: used to assemble final file.
    fn finalize(
        &self,
        imports: Vec<TokenStream>,
        args_structs: Vec<TokenStream>,
        functions: Vec<TokenStream>,
        methods: Vec<TokenStream>,
        additional: TokenStream,
    ) -> TokenStream;
}
