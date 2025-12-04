//! Differential Fuzzing step.

use anyhow::anyhow;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};

use crate::{
    check::{CheckResult, Checker, Component},
    config::DiffFuzzConfig,
    defs::{CommonFunction, Path, Precondition},
    generate::{FunctionCollection, HarnessBackend, HarnessGenerator},
    utils::{create_harness_project, run_command},
};

/// Differential fuzzing harness generator backend.
struct DFHarnessBackend {
    /// Use preconditions.
    use_preconditions: bool,
    /// Catch panic unwind.
    catch_panic: bool,
    /// Enable log in fuzzing harness
    harness_log: bool,
}

impl HarnessBackend for DFHarnessBackend {
    fn arg_struct_attrs(&self) -> TokenStream {
        quote! {
            #[derive(Debug, serde::Deserialize)]
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

        // If a precondition is provided, generate precondition check code before function call
        let precondition = self
            .use_preconditions
            .then(|| {
                precondition.map(|pre| {
                    let check_fn_name = pre.checker_name();
                    quote! {
                        if !#check_fn_name(#(function_arg_struct.#function_args),*) {
                            return true;
                        }
                    }
                })
            })
            .flatten();
        // Function call with panic catch if enabled
        let fn_call = |mod_: TokenStream| {
            if self.catch_panic {
                quote! {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #mod_::#fn_name(#(function_arg_struct.#function_args),*)
                    }))
                    .map_err(|_| ())
                }
            } else {
                quote! {
                    #mod_::#fn_name(#(function_arg_struct.#function_args),*)
                }
            }
        };
        let r1_call = fn_call(quote! {mod1});
        let r2_call = fn_call(quote! {mod2});

        // Error report message
        let err_report = quote! {
            outputln!("MISMATCH: {}", #fn_name_string);
            outputln!("function: {:?}", function_arg_struct);
        };
        // Return value check code
        let retv_check = quote! {
            if r1 != r2 {
                #err_report
                return false;
            }
        };

        quote! {
            #[inline(always)]
            fn #test_fn_name(input: &[u8]) -> bool {
                // Function arguments
                let function_arg_struct = match postcard::from_bytes::<#function_arg_struct>(&input[..]) {
                    Ok(args) => args,
                    Err(_) => return true,
                };
                // Precondition check
                #precondition
                // Do function call
                let r1 = #r1_call;
                let r2 = #r2_call;

                #retv_check
                true
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
        let fn_name_string = fn_name.to_string();
        let constr_name = &constructor.metadata.name;

        // Test function name
        let test_fn_name = format_ident!("check_{}", fn_name.to_ident());
        // Method argument struct name
        let method_arg_struct = format_ident!("Args{}", fn_name.to_ident());
        // Constructor argument struct name
        let constructor_arg_struct = format_ident!("Args{}", constr_name.to_ident());

        // If a precondition is provided, generate precondition check code before method call
        let precondition = self
            .use_preconditions
            .then(|| {
                precondition.map(|pre| {
                    let check_fn_name = pre.checker_name();
                    quote! {
                        if !s2.#check_fn_name(#(method_arg_struct.#method_args),*) {
                            return true;
                        }
                    }
                })
            })
            .flatten();
        // Constructor call with panic catch if enabled
        let constr_call = |mod_: TokenStream| {
            if self.catch_panic {
                quote! {
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #mod_::#constr_name(#(constr_arg_struct.#constructor_args),*)
                    })) {
                        Ok(s) => s,
                        Err(_) => return true,
                    }
                }
            } else {
                quote! {
                    #mod_::#constr_name(#(constr_arg_struct.#constructor_args),*)
                }
            }
        };
        let s1_construct = constr_call(quote! {mod1});
        let s2_construct = constr_call(quote! {mod2});
        // Method call with panic catch if enabled
        let method_call = |mod_: TokenStream, s: TokenStream| {
            if self.catch_panic {
                quote! {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #mod_::#fn_name(
                            #receiver_prefix #s, #(method_arg_struct.#method_args),*
                        )
                    }))
                    .map_err(|_| ())
                }
            } else {
                quote! {
                    #mod_::#fn_name(
                        #receiver_prefix #s, #(method_arg_struct.#method_args),*
                    )
                }
            }
        };
        let r1_call = method_call(quote! {mod1}, quote! {s1});
        let r2_call = method_call(quote! {mod2}, quote! {s2});

        // Error report message
        let err_report = quote! {
            outputln!("MISMATCH: {}", #fn_name_string);
            outputln!("contructor: {:?}", constr_arg_struct);
            outputln!("method: {:?}", method_arg_struct);
        };
        // Return value check code
        let retv_check = quote! {
            if r1 != r2 {
                #err_report
                return false;
            }
        };
        // If a getter is provided, generate state check code after method call
        let state_check = getter.map(|getter| {
            let getter = &getter.metadata.signature.0.ident;
            quote! {
                if s1.#getter() != s2.#getter() {
                    #err_report
                    return false;
                }
            }
        });

        quote! {
            #[inline(always)]
            fn #test_fn_name(input: &[u8]) -> bool {
                // Constructor arguments
                let (constr_arg_struct, remain) = match postcard::take_from_bytes::<#constructor_arg_struct>(
                    &input[..]
                ) {
                    Ok((args, remain)) => (args, remain),
                    Err(_) => return true,
                };
                // Method arguments
                let method_arg_struct = match postcard::from_bytes::<#method_arg_struct>(&remain[..]) {
                    Ok(args) => args,
                    Err(_) => return true,
                };

                // Construct s1 and s2
                let mut s1 = #s1_construct;
                let mut s2 = #s2_construct;
                // Precondition check
                #precondition
                // Do method call
                let r1 = #r1_call;
                let r2 = #r2_call;

                #retv_check
                #state_check
                true
            }
        }
    }

    fn additional_code(&self, collection: &FunctionCollection) -> TokenStream {
        // Generate dispatch function as additional code
        let test_fns = collection
            .functions
            .iter()
            .map(|func| format!("check_{}", func.metadata.name.to_ident()))
            .chain(
                collection
                    .methods
                    .iter()
                    .map(|method| format!("check_{}", method.metadata.name.to_ident())),
            )
            .collect::<Vec<_>>();

        let fn_count = test_fns.len();
        let match_arms = test_fns.iter().enumerate().map(|(i, name)| {
            let fn_name = format_ident!("{}", name);
            let i = i as u8;
            quote! {
                #i => #fn_name(&input[1..]),
            }
        });
        quote! {
            fn run_harness(input: &[u8]) -> bool {
                if input.len() == 0 {
                    return true;
                }
                let fn_id = input[0] % #fn_count as u8;
                match fn_id {
                    #(#match_arms)*
                    _ => true,
                }
            }
        }
    }

    fn finalize(
        &self,
        imports: Vec<TokenStream>,
        args_structs: Vec<TokenStream>,
        functions: Vec<TokenStream>,
        methods: Vec<TokenStream>,
        additional: TokenStream,
    ) -> TokenStream {
        let log_utils = if self.harness_log {
            quote! {
                // Harness logging utils
                use std::io::Write;
                static HARNESS_OUTPUT: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();
                fn init_harness_output() {
                    HARNESS_OUTPUT.set(std::fs::File::create("harness_output.log").unwrap()).unwrap();
                }
                fn get_harness_output() -> &'static std::fs::File {
                    HARNESS_OUTPUT.get().expect("not initialized")
                }
                macro_rules! outputln {
                    ($($arg:tt)*) => {
                        writeln!(get_harness_output(), $($arg)*).unwrap();
                    };
                }
            }
        } else {
            quote! {
                macro_rules! outputln {
                    ($($arg:tt)*) => {};
                }
            }
        };
        let init_log = self.harness_log.then(|| {
            quote! {
                init_harness_output();
            }
        });

        quote! {
            #![allow(unused)]
            #![allow(non_snake_case)]
            #![allow(non_camel_case_types)]
            mod mod1;
            mod mod2;
            #(#imports)*

            // Harness logging utils
            #log_utils
            fn main() {
                #init_log
                afl::fuzz_nohook!(|data: &[u8]| {
                    if !run_harness(data) {
                        panic!("Harness reported failure for input: {:?}", data);
                    }
                });
            }

            #(#args_structs)*
            #(#functions)*
            #(#methods)*
            #additional
        }
    }
}

/// Differential fuzzing harness generator.
type DFHarnessGenerator = HarnessGenerator<DFHarnessBackend>;

/// Differential Fuzzing step.
pub struct DifferentialFuzzing {
    config: DiffFuzzConfig,
}

impl DifferentialFuzzing {
    /// Create a new Differential Fuzzing component with the given configuration.
    pub fn new(config: DiffFuzzConfig) -> Self {
        Self { config }
    }

    /// Return the functions that are checked in the harness.
    fn checked_functions(&self, checker: &Checker) -> Vec<Path> {
        let mut collection = FunctionCollection::new(
            checker.under_checking_funcs.clone(),
            checker.constructors.clone(),
            checker.getters.clone(),
            checker.preconditions.clone(),
        );
        collection.remove_methods_without_constructors();
        collection.remove_unused_constructors_and_getters();
        collection
            .functions
            .iter()
            .map(|f| f.metadata.name.clone())
            .chain(collection.methods.iter().map(|f| f.metadata.name.clone()))
            .collect::<Vec<_>>()
    }

    /// Generate the fuzzing harness.
    fn generate_harness(&self, checker: &Checker) -> TokenStream {
        let generator = DFHarnessGenerator::new(
            checker,
            DFHarnessBackend {
                use_preconditions: self.config.use_preconditions,
                catch_panic: self.config.catch_panic,
                harness_log: self.config.harness_log,
            },
        );
        generator.generate_harness()
    }

    /// Create a cargo project for LibAFL harness.
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
serde = "*"
postcard = "*"
afl = "*"
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

    /// Prepare initial inputs for the fuzzer.
    fn prepare_initial_inputs(&self) -> anyhow::Result<()> {
        let inputs_dir = format!("{}/in", &self.config.harness_path);
        std::fs::create_dir_all(&inputs_dir)
            .map_err(|_| anyhow!("Failed to create inputs directory"))?;

        for i in 0..self.config.initial_inputs {
            let mut file = std::fs::File::create(format!("{}/input{}", inputs_dir, i))
                .map_err(|_| anyhow!("Failed to create initial input file"))?;
            // Generate random input data
            let mut buf = Vec::with_capacity(self.config.input_len);
            for _ in 0..(self.config.input_len) {
                buf.push(rand::random::<u8>());
            }
            file.write_all(&buf)
                .map_err(|_| anyhow!("Failed to write initial input file"))?;
        }

        Ok(())
    }

    /// Execute custom command before fuzzing
    fn execute_pre_fuzz_cmd(&self) -> anyhow::Result<()> {
        if let Some(cmd) = &self.config.pre_fuzz_cmd {
            let status = run_command("sh", &["-c", cmd], None, None)?;
            if !status.success() {
                return Err(anyhow!("Pre-fuzz command failed with status: {}", status));
            }
        }
        Ok(())
    }

    /// Run the fuzzer on the harness project.
    fn run_fuzzer(&self) -> anyhow::Result<()> {
        let build_status = run_command(
            "cargo",
            &["afl", "build", "--release"],
            None,
            Some(&self.config.harness_path),
        )?;
        if build_status.code() == Some(101) {
            return Err(anyhow!("Command failed due to compilation error"));
        }

        let _fuzz_status = run_command(
            "cargo",
            &[
                "afl",
                "fuzz",
                "-i",
                "in",
                "-o",
                "out",
                "-E",
                self.config.executions.to_string().as_str(),
                "target/release/harness",
            ],
            None,
            Some(&self.config.harness_path),
        )?;
        std::fs::copy(
            format!("{}/harness_output.log", self.config.harness_path),
            &self.config.output_path,
        )
        .map_err(|e| anyhow!("Failed to copy harness output log: {}", e))?;

        Ok(())
    }

    /// Analyze the fuzzer output and return the functions that are not checked.
    fn analyze_fuzzer_output(&self, functions: &[Path]) -> CheckResult {
        let mut res = CheckResult {
            status: Ok(()),
            ok: functions.to_vec(),
            fail: vec![],
        };

        let re = Regex::new(r"MISMATCH:\s*(\S+)").unwrap();
        let file = std::fs::File::open(&self.config.output_path).unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            if let Some(caps) = re.captures(&line.unwrap()) {
                let func_name = caps[1].to_string();
                if let Some(i) = res.ok.iter().position(|f| f.to_string() == func_name) {
                    res.ok.swap_remove(i);
                    res.fail.push(Path::from_str(&func_name));
                }
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

impl Component for DifferentialFuzzing {
    fn name(&self) -> &str {
        "Differential Fuzzing"
    }

    fn is_formal(&self) -> bool {
        false
    }

    fn note(&self) -> Option<&str> {
        Some("Using differential fuzzing to find inconsistencies.")
    }

    fn run(&self, checker: &Checker) -> CheckResult {
        if self.config.gen_harness {
            let harness = self.generate_harness(checker);
            let res = self.create_harness_project(checker, harness);
            if let Err(e) = res {
                return CheckResult::failed(e);
            }
        }
        // Note: if using existing harness, the checked functions may be different from
        // generated harness, but we still use the functions from checker for analysis.
        let functions = self.checked_functions(checker);

        let res = self.prepare_initial_inputs();
        if let Err(e) = res {
            return CheckResult::failed(e);
        }
        let res = self.execute_pre_fuzz_cmd();
        if let Err(e) = res {
            return CheckResult::failed(e);
        }
        let res = self.run_fuzzer();
        if let Err(e) = res {
            return CheckResult::failed(e);
        }
        let check_res = self.analyze_fuzzer_output(&functions);

        if !self.config.keep_harness {
            if let Err(e) = self.remove_harness_project() {
                return CheckResult::failed(e);
            }
        }
        if !self.config.keep_output {
            if let Err(e) = self.remove_output_file() {
                return CheckResult::failed(anyhow!("Failed to remove output file: {}", e));
            }
        }

        check_res
    }
}
