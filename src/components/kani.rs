//! Use model-checker Kani to check function equivalence

use anyhow::anyhow;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use std::{io::BufRead, str::FromStr};

use crate::{
    check::{CheckResult, Checker, Component},
    config::KaniConfig,
    defs::{CommonFunction, Path, Precondition},
    generate::{HarnessBackend, HarnessGenerator},
    utils::{create_harness_project, run_command},
};

/// Kani harness generator backend.
struct KaniHarnessBackend {
    /// Use preconditions.
    use_preconditions: bool,
    /// Loop unwind limit.
    loop_unwind: Option<u32>,
}

impl HarnessBackend for KaniHarnessBackend {
    fn arg_struct_attrs(&self) -> TokenStream {
        quote! {
            #[derive(Debug, kani::Arbitrary)]
        }
    }

    fn make_harness_for_function(
        &self,
        function: &CommonFunction,
        function_args: &[TokenStream],
        precondition: Option<&Precondition>,
    ) -> TokenStream {
        let fn_name = &function.metadata.name;

        // Test function name
        let test_fn_name = format_ident!("check_{}", fn_name.to_ident());
        // Function argument struct name
        let function_arg_struct = format_ident!("Args{}", fn_name.to_ident());

        // If precondition is present, we may need to add assume code
        let precondition = self
            .use_preconditions
            .then(|| {
                precondition.map(|pre| {
                    let check_fn_name = pre.checker_name();
                    quote! {
                        kani::assume(#check_fn_name(#(function_arg_struct.#function_args),*));
                    }
                })
            })
            .flatten();
        // If loop unwind is specified, add unwind attribute
        let unwind_attr = self.loop_unwind.map(|unwind| {
            let unwind = TokenStream::from_str(&unwind.to_string()).unwrap();
            quote! {
                #[kani::unwind(#unwind)]
            }
        });

        quote! {
            #[cfg(kani)]
            #[kani::proof]
            #[allow(non_snake_case)]
            #unwind_attr
            pub fn #test_fn_name() {
                let function_arg_struct = kani::any::<#function_arg_struct>();
                // Precondition assume
                #precondition
                // Function call
                let r1 = mod1::#fn_name(#(function_arg_struct.#function_args),*);
                let r2 = mod2::#fn_name(#(function_arg_struct.#function_args),*);
                assert!(r1 == r2);
            }
        }
    }

    fn make_harness_for_method(
        &self,
        method: &CommonFunction,
        constructor: &CommonFunction,
        getter: Option<&CommonFunction>,
        method_args: &[TokenStream],
        constructor_args: &[TokenStream],
        receiver_prefix: TokenStream,
        precondition: Option<&Precondition>,
    ) -> TokenStream {
        let fn_name = &method.metadata.name;
        let constr_name = &constructor.metadata.name;

        // Test function name
        let test_fn_name = format_ident!("check_{}", fn_name.to_ident());
        // Method argument struct name
        let method_arg_struct = format_ident!("Args{}", fn_name.to_ident());
        // Constructor argument struct name
        let constructor_arg_struct = format_ident!("Args{}", constr_name.to_ident());

        // If a getter is provided, generate state check code after method call
        let state_check = getter.map(|getter| {
            let getter = &getter.metadata.signature.0.ident;
            quote! {
                assert!(s1.#getter() == s2.#getter());
            }
        });

        // If precondition is present, we may need to add assume code
        let precondition = self
            .use_preconditions
            .then(|| {
                precondition.map(|pre| {
                    let check_fn_name = pre.checker_name();
                    quote! {
                        kani::assume(s2.#check_fn_name(#(method_arg_struct.#method_args),*));
                    }
                })
            })
            .flatten();
        // If loop unwind is specified, add unwind attribute
        let unwind_attr = self.loop_unwind.map(|unwind| {
            let unwind = TokenStream::from_str(&unwind.to_string()).unwrap();
            quote! {
                #[kani::unwind(#unwind)]
            }
        });

        quote! {
            #[cfg(kani)]
            #[kani::proof]
            #[allow(non_snake_case)]
            #unwind_attr
            pub fn #test_fn_name() {
                let constr_arg_struct = kani::any::<#constructor_arg_struct>();
                // Construct s1 and s2
                let mut s1 = mod1::#constr_name(#(constr_arg_struct.#constructor_args),*);
                let mut s2 = mod2::#constr_name(#(constr_arg_struct.#constructor_args),*);

                let method_arg_struct = kani::any::<#method_arg_struct>();
                // Precondition assume
                #precondition
                // Do method call
                let r1 = mod1::#fn_name(#receiver_prefix s1, #(method_arg_struct.#method_args),*);
                let r2 = mod2::#fn_name(#receiver_prefix s2, #(method_arg_struct.#method_args),*);

                assert!(r1 == r2);
                #state_check
            }
        }
    }

    fn finalize(
        &self,
        imports: Vec<TokenStream>,
        args_structs: Vec<TokenStream>,
        functions: Vec<TokenStream>,
        methods: Vec<TokenStream>,
        _additional: TokenStream,
    ) -> TokenStream {
        quote! {
            #![allow(unused)]
            #![allow(non_snake_case)]
            #![allow(non_camel_case_types)]
            mod mod1;
            mod mod2;

            #(#imports)*
            #(#args_structs)*
            #(#functions)*
            #(#methods)*

            fn main() {}
        }
    }
}

/// Kani harness generator.
type KaniHarnessGenerator = HarnessGenerator<KaniHarnessBackend>;

/// Kani step: use Kani model-checker to check function equivalence.
pub struct Kani {
    config: KaniConfig,
}

impl Kani {
    /// Create a new Kani component with the given configuration.
    pub fn new(config: KaniConfig) -> Self {
        Self { config }
    }

    /// Generate harness code for Kani.
    fn generate_harness(&self, checker: &Checker) -> TokenStream {
        let generator = KaniHarnessGenerator::new(
            checker,
            KaniHarnessBackend {
                use_preconditions: self.config.use_preconditions,
                loop_unwind: self.config.loop_unwind,
            },
        );
        generator.generate_harness()
    }

    /// Create a cargo project for Kani harness.
    fn create_harness_project(
        &self,
        checker: &Checker,
        harness: TokenStream,
    ) -> anyhow::Result<()> {
        let toml = r#"
[package]
name = "harness"
version = "0.1.0"
edition = "2024"

[dev-dependencies]
kani = "*"
"#;
        create_harness_project(
            &self.config.harness_path,
            &checker.src1.content,
            &checker.src2.content,
            &harness.to_string(),
            toml,
            false,
        )
    }

    /// Run Kani and save the output.
    fn run_kani(&self) -> anyhow::Result<()> {
        let timeout_secs = self.config.timeout_secs;
        let status = run_command(
            "cargo",
            &[
                "kani",
                "-Z",
                "unstable-options",
                "--harness-timeout",
                &format!("{}s", timeout_secs),
            ],
            Some(&self.config.output_path),
            Some(&self.config.harness_path),
        )?;

        if status.code() == Some(101) {
            return Err(anyhow!("Command failed due to compilation error"));
        }
        Ok(())
    }

    /// Analyze Kani output from "kani.tmp".
    fn analyze_kani_output(&self) -> CheckResult {
        let mut res = CheckResult {
            status: Ok(()),
            ok: vec![],
            fail: vec![],
        };

        let re = Regex::new(r"Checking harness check_([0-9a-zA-Z_]+)\.").unwrap();
        let file = std::fs::File::open(&self.config.output_path).unwrap();
        let reader = std::io::BufReader::new(file);
        let mut func_name: Option<String> = None;

        for line in reader.lines() {
            let line = line.unwrap();
            if let Some(caps) = re.captures(&line) {
                func_name = Some(caps[1].replace("___", "::"));
            }
            if line.contains("VERIFICATION:- SUCCESSFUL") && func_name.is_some() {
                res.ok.push(Path::from_str(&func_name.take().unwrap()));
            } else if line.contains("VERIFICATION:- FAILED") && func_name.is_some() {
                res.fail.push(Path::from_str(&func_name.take().unwrap()));
            }
        }

        res
    }

    /// Remove the harness project.
    fn remove_harness_project(&self) -> anyhow::Result<()> {
        std::fs::remove_dir_all(&self.config.harness_path)
            .map_err(|_| anyhow!("Failed to remove harness project"))
    }

    /// Remove the output file.
    fn remove_output_file(&self) -> anyhow::Result<()> {
        std::fs::remove_file(&self.config.output_path)
            .map_err(|_| anyhow!("Failed to remove output file"))
    }
}

impl Component for Kani {
    fn name(&self) -> &str {
        "Kani"
    }

    fn is_formal(&self) -> bool {
        true
    }

    fn note(&self) -> Option<&str> {
        Some("Use Kani model-checker to check function consistency")
    }

    fn run(&self, checker: &Checker) -> CheckResult {
        if self.config.gen_harness {
            let harness = self.generate_harness(checker);
            let res = self.create_harness_project(checker, harness);
            if let Err(e) = res {
                return CheckResult::failed(e);
            }
        }
        let res = self.run_kani();
        if let Err(e) = res {
            return CheckResult::failed(e);
        }
        let check_res = self.analyze_kani_output();
        if !self.config.keep_harness {
            if let Err(e) = self.remove_harness_project() {
                return CheckResult::failed(e);
            }
        }
        if !self.config.keep_output {
            if let Err(e) = self.remove_output_file() {
                return CheckResult::failed(e);
            }
        }

        check_res
    }
}
