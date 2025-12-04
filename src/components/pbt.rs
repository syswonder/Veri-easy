//! Property-based testing step.

use anyhow::anyhow;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use std::{
    io::{BufRead, BufReader},
    str::FromStr,
};

use crate::{
    check::{CheckResult, Checker, Component},
    config::PBTConfig,
    defs::{CommonFunction, Path, Precondition},
    generate::{HarnessBackend, HarnessGenerator},
    utils::{create_harness_project, run_command},
};

/// PBT harness generator backend.
struct PBTHarnessBackend {
    /// Number of test cases.
    cases: usize,
    /// Timeout in seconds.
    timeout_secs: u64,
    /// Use preconditions.
    use_preconditions: bool,
}

impl HarnessBackend for PBTHarnessBackend {
    fn arg_struct_attrs(&self) -> TokenStream {
        quote! {
            #[derive(Debug)]
            #[cfg_attr(test, derive(proptest_derive::Arbitrary))]
        }
    }

    fn make_harness_for_function(
        &self,
        function: &CommonFunction,
        function_args: &[TokenStream],
        precondition: Option<&Precondition>,
    ) -> TokenStream {
        let fn_name = &function.metadata.name;
        let fn_name_string = fn_name.to_string();

        // Test function name
        let test_fn_name = format_ident!("check_{}", fn_name.to_ident());
        // Function argument struct name
        let function_arg_struct = format_ident!("Args{}", fn_name.to_ident());

        // If a precondition is provided, add assume statements before function call
        let precondition = self
            .use_preconditions
            .then(|| {
                precondition.map(|pre| {
                    let check_fn_name = pre.checker_name();
                    quote! {
                        prop_assume!(#check_fn_name(#(function_arg_struct.#function_args),*));
                    }
                })
            })
            .flatten();
        // Error report message
        let err_report = quote! {
            println!("MISMATCH {}", #fn_name_string);
            println!("function: {:?}", function_arg_struct);
        };
        // Return value check code
        let retv_check = quote! {
            if r1 != r2 {
                #err_report
                assert!(false);
            }
        };

        quote! {
            #[test]
            fn #test_fn_name(function_arg_struct in any::<#function_arg_struct>()) {
                // Precondition assume
                #precondition

                // Function call
                let r1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    mod1::#fn_name(#(function_arg_struct.#function_args),*)
                }))
                .map_err(|_| ());
                let r2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    mod2::#fn_name(#(function_arg_struct.#function_args),*)
                }))
                .map_err(|_| ());

                #retv_check
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
        let fn_name_string = fn_name.to_string();

        // Test function name
        let test_fn_name = format_ident!("check_{}", fn_name.to_ident());
        // Method argument struct name
        let method_arg_struct = format_ident!("Args{}", fn_name.to_ident());
        // Constructor argument struct name
        let constructor_arg_struct = format_ident!("Args{}", constr_name.to_ident());

        // If a precondition is provided, add assume statements before method call
        let precondition = self.use_preconditions.then(|| {
            precondition.map(|pre| {
                let check_fn_name = pre.checker_name();
                quote! {
                    if !s2.#check_fn_name(#(method_arg_struct.#method_args),*) {
                        return Ok(());
                    }
                }
            })
        });

        // Error report message
        let err_report = quote! {
            println!("MISMATCH: {}", #fn_name_string);
            println!("method: {:?}", method_arg_struct);
        };
        // Return value check code
        let retv_check = quote! {
            if r1 != r2 {
                #err_report
                assert!(false);
            }
        };
        // If a getter is provided, generate state check code after method call
        let state_check = getter.map(|getter| {
            let getter = &getter.metadata.signature.0.ident;
            quote! {
                if s1.#getter() != s2.#getter() {
                    #err_report
                    assert!(false);
                }
            }
        });

        quote! {
            #[test]
            fn #test_fn_name(
                constr_arg_struct in any::<#constructor_arg_struct>(),
                method_arg_struct in any::<#method_arg_struct>(),
            ) {
                // Construct s1 and s2
                let mut s1 = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    mod1::#constr_name(#(constr_arg_struct.#constructor_args),*)
                })) {
                    Ok(s) => s,
                    Err(_) => return Ok(()),
                };
                let mut s2 = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    mod2::#constr_name(#(constr_arg_struct.#constructor_args),*)
                })) {
                    Ok(s) => s,
                    Err(_) => return Ok(()),
                };

                // Precondition assume
                #precondition

                // Method call
                let r1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    mod1::#fn_name(
                        #receiver_prefix s1, #(method_arg_struct.#method_args),*
                    )
                }))
                .map_err(|_| ());
                let r2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    mod2::#fn_name(
                        #receiver_prefix s2, #(method_arg_struct.#method_args),*
                    )
                }))
                .map_err(|_| ());

                #retv_check
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
        let cases = TokenStream::from_str(&self.cases.to_string()).unwrap();
        let timeout = TokenStream::from_str(&(self.timeout_secs * 1000).to_string()).unwrap();
        quote! {
            #![allow(unused)]
            #![allow(non_snake_case)]
            #![allow(non_camel_case_types)]
            mod mod1;
            mod mod2;
            use proptest::prelude::*;

            #(#imports)*
            #(#args_structs)*
            proptest! {
                #![proptest_config(ProptestConfig {
                    cases: #cases,
                    timeout: #timeout,
                    .. ProptestConfig::default()
                })]
                #(#functions)*
                #(#methods)*
            }
            fn main() {}
        }
    }
}

/// PBT harness generator.
type PBTHarnessGenerator = HarnessGenerator<PBTHarnessBackend>;

/// Property-based testing step using Proptest.
pub struct PropertyBasedTesting {
    config: PBTConfig,
}

impl PropertyBasedTesting {
    /// Create a new Property-Based Testing component with the given configuration.
    pub fn new(config: PBTConfig) -> Self {
        Self { config }
    }

    /// Generate the PBT harness.
    fn generate_harness(&self, checker: &Checker) -> TokenStream {
        let generator = PBTHarnessGenerator::new(
            checker,
            PBTHarnessBackend {
                cases: self.config.test_cases,
                timeout_secs: self.config.timeout_secs,
                use_preconditions: self.config.use_preconditions,
            },
        );
        generator.generate_harness()
    }

    /// Create a cargo project for proptest harness.
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

[dependencies]
proptest = "1.9"
proptest-derive = "0.2.0"
"#;
        // Set RUST_MIN_STACK to 16MB to avoid stack overflow in proptest
        let config = r#"
[env]
RUST_MIN_STACK = "16777216"
"#;
        create_harness_project(
            &self.config.harness_path,
            &checker.src1.content,
            &checker.src2.content,
            &harness.to_string(),
            toml,
            false,
        )?;
        std::fs::create_dir_all(format!("{}/.cargo", &self.config.harness_path))?;
        std::fs::write(
            format!("{}/.cargo/config.toml", &self.config.harness_path),
            config,
        )?;
        Ok(())
    }

    /// Run libAFL fuzzer and save the ouput in "df.tmp".
    fn run_test(&self) -> anyhow::Result<()> {
        run_command(
            "cargo",
            &["test"],
            Some(&self.config.output_path),
            Some(&self.config.harness_path),
        )?;
        Ok(())
    }

    /// Analyze the fuzzer output and return the functions that are not checked.
    fn analyze_pbt_output(&self) -> CheckResult {
        let mut res = CheckResult {
            status: Ok(()),
            ok: vec![],
            fail: vec![],
        };

        let re_ok = Regex::new(r"test check_\s*(\S+) ... ok").unwrap();
        let re_fail = Regex::new(r"test check_\s*(\S+) ... FAILED").unwrap();

        let file = std::fs::File::open(&self.config.output_path).unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            if let Some(caps) = re_ok.captures(line.as_ref().unwrap()) {
                let func_name = caps[1].to_string().replace("___", "::");
                res.ok.push(Path::from_str(&func_name));
            }
            if let Some(caps) = re_fail.captures(line.as_ref().unwrap()) {
                let func_name = caps[1].to_string().replace("___", "::");
                res.fail.push(Path::from_str(&func_name));
            }
        }

        res
    }

    /// Remove the harness project.
    fn remove_harness_project(&self) -> anyhow::Result<()> {
        std::fs::remove_dir_all(&self.config.harness_path)
            .map_err(|_| anyhow!("Failed to remove harness file"))
    }

    /// Remove the output file.
    fn remove_output_file(&self) -> anyhow::Result<()> {
        std::fs::remove_file(&self.config.output_path)
            .map_err(|_| anyhow!("Failed to remove output file"))
    }
}

impl Component for PropertyBasedTesting {
    fn name(&self) -> &str {
        "Property-Based Testing"
    }

    fn is_formal(&self) -> bool {
        false
    }

    fn note(&self) -> Option<&str> {
        Some("Uses Proptest to generate inputs and compare function behaviors.")
    }

    fn run(&self, checker: &Checker) -> CheckResult {
        if self.config.gen_harness {
            let harness = self.generate_harness(checker);
            let res = self.create_harness_project(checker, harness);
            if let Err(e) = res {
                return CheckResult::failed(e);
            }
        }

        let res = self.run_test();
        if let Err(e) = res {
            return CheckResult::failed(e);
        }
        let check_res = self.analyze_pbt_output();

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
