//! Alive2 step: use alive-tv to check function equivalence.

use anyhow::{Result, anyhow};
use std::{io::BufRead, process::Command};
use syn::{
    Attribute, File, ImplItemFn, ItemFn, ItemImpl,
    visit_mut::{self, VisitMut},
};

use crate::{
    check::{CheckResult, Checker, Component},
    config::Alive2Config,
    defs::Path,
};

/// Alive2 step: use alive-tv to check function equivalence.
pub struct Alive2 {
    config: Alive2Config,
}

impl Alive2 {
    /// Create a new Alive2 component with the given configuration.
    pub fn new(config: Alive2Config) -> Self {
        Self { config }
    }

    /// Compile the source file to LLVM IR with exported function names.
    fn compile_to_llvm_ir(&self, src_path: &str, output_path: &str) -> anyhow::Result<()> {
        let original =
            std::fs::read_to_string(src_path).map_err(|_| anyhow!("Failed to read source"))?;
        // Add #[export_name = "..."] to all functions, save to tmp file
        let exported = export_functions(&original)?;
        let tmp_path = "tmp.rs";
        std::fs::write(&tmp_path, exported).map_err(|_| anyhow!("Failed to write tmp file"))?;

        Command::new("rustc")
            .args([
                "--emit=llvm-ir",
                "--crate-type=lib",
                tmp_path,
                "-o",
                output_path,
            ])
            .stderr(std::fs::File::open("/dev/null").unwrap())
            .status()
            .map(|_| ())
            .map_err(|_| anyhow!("Failed to compile to llvm-ir"))?;
        std::fs::remove_file(tmp_path).map_err(|_| anyhow!("Failed to remove tmp file"))
    }

    /// Remove the generated LLVM IR file.
    fn remove_llvm_ir(&self, ir_path: &str) -> anyhow::Result<()> {
        std::fs::remove_file(ir_path).map_err(|_| anyhow!("Failed to remove llvm-ir"))
    }

    /// Run alive-tv on the two LLVM IR files and save the output.
    fn run_alive2(&self, ir1: &str, ir2: &str, output_path: &str) -> anyhow::Result<()> {
        let output_file =
            std::fs::File::create(output_path).map_err(|_| anyhow!("Failed to create tmp file"))?;
        Command::new(self.config.alive2_path.clone())
            .args([ir1, ir2])
            .stdout(output_file)
            .status()
            .map_err(|_| anyhow!("Failed to run alive-tv"))?;
        Ok(())
    }

    /// Analyze the output of alive-tv and produce a CheckResult.
    fn analyze_alive2_output(&self, output_path: &str) -> CheckResult {
        let mut res = CheckResult {
            status: Ok(()),
            ok: vec![],
            fail: vec![],
        };

        let file = std::fs::File::open(output_path).unwrap();
        let reader = std::io::BufReader::new(file);
        let mut func_name: Option<String> = None;

        for line in reader.lines() {
            let line = line.unwrap();
            if line.starts_with("define") {
                if func_name.is_none() {
                    let at = line.find("@").unwrap();
                    let parenthese = line.find('(').unwrap();
                    func_name = Some(line[at + 1..parenthese].to_string().replace("___", "::"));
                }
            } else if line.starts_with("Transformation seems to be correct!") {
                res.ok.push(Path::from_str(&func_name.take().unwrap()));
            } else if line.starts_with("ERROR") {
                func_name = None;
            }
        }

        res
    }

    /// Remove the alive2 output file.
    fn remove_alive2_output(&self) -> anyhow::Result<()> {
        std::fs::remove_file(&self.config.output_path)
            .map_err(|_| anyhow!("Failed to remove alive2 output file"))
    }
}

impl Component for Alive2 {
    fn name(&self) -> &str {
        "Alive2"
    }

    fn is_formal(&self) -> bool {
        true
    }

    fn note(&self) -> Option<&str> {
        Some("Use alive-tv to check function equivalence")
    }

    fn run(&self, checker: &Checker) -> CheckResult {
        let out1 = "alive2_1.ll";
        let out2 = "alive2_2.ll";

        let res = self.compile_to_llvm_ir(&checker.src1.path, out1);
        if let Err(e) = res {
            return CheckResult::failed(e);
        }
        let res = self.compile_to_llvm_ir(&checker.src2.path, out2);
        if let Err(e) = res {
            return CheckResult::failed(e);
        }

        let res = self.run_alive2(out1, out2, &self.config.output_path);
        if let Err(e) = res {
            return CheckResult::failed(e);
        }
        let check_res = self.analyze_alive2_output(&self.config.output_path);

        if let Err(e) = self.remove_llvm_ir(out1) {
            return CheckResult::failed(e);
        }
        if let Err(e) = self.remove_llvm_ir(out2) {
            return CheckResult::failed(e);
        }
        if !self.config.keep_output {
            if let Err(e) = self.remove_alive2_output() {
                return CheckResult::failed(e);
            }
        }

        check_res
    }
}

/// Visitor that sets `#[export_name = "..."]` on functions and impl methods.
struct FnExporter {
    scope_stack: Vec<String>,
}

impl FnExporter {
    fn new() -> Self {
        Self {
            scope_stack: Vec::new(),
        }
    }
    fn concat_name(&self, name: &str) -> String {
        if self.scope_stack.is_empty() {
            name.to_string()
        } else {
            self.scope_stack.join("___") + "___" + name
        }
    }
}

impl VisitMut for FnExporter {
    fn visit_item_fn_mut(&mut self, node: &mut ItemFn) {
        if node.sig.generics.lt_token.is_none() {
            let name = self.concat_name(&node.sig.ident.to_string());
            let attr: Attribute = syn::parse_quote!(#[export_name = #name]);
            node.attrs.push(attr);
        }
        // skip function with generic params
        visit_mut::visit_item_fn_mut(self, node);
    }

    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        self.scope_stack.push(i.ident.to_string());
        visit_mut::visit_item_mod_mut(self, i);
        self.scope_stack.pop();
    }

    fn visit_item_impl_mut(&mut self, node: &mut ItemImpl) {
        if node.generics.lt_token.is_none() {
            self.scope_stack.push(type_to_string(&node.self_ty, "___"));
            visit_mut::visit_item_impl_mut(self, node);
            self.scope_stack.pop();
        }
        // skip impl block with generic params
    }

    fn visit_impl_item_fn_mut(&mut self, node: &mut ImplItemFn) {
        let name = self.concat_name(&node.sig.ident.to_string());
        let attr: Attribute = syn::parse_quote!(#[export_name = #name]);
        node.attrs.push(attr);
        visit_mut::visit_impl_item_fn_mut(self, node);
    }
}

/// Add `#[export_name = "..."]` to all functions and impl methods
fn export_functions(src: &str) -> Result<String> {
    let mut syntax: File = syn::parse_file(src)?;
    let mut exporter = FnExporter::new();
    exporter.visit_file_mut(&mut syntax);
    Ok(prettyplease::unparse(&syntax))
}

/// Convert a type to a string
fn type_to_string(ty: &syn::Type, sep: &str) -> String {
    match ty {
        syn::Type::Path(tp) => tp
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join(sep),
        _ => "unsupported".to_owned(),
    }
}
