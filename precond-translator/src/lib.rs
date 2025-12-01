use crate::{
    collect::{PrecondCollector, SpecFunctionCollector},
    generate::CodeGenerator,
};
use verus_syn::File;

mod ast;
mod collect;
mod generate;
mod visit;

/// Collect preconditions and spec functions/methods from a Verus file, then create a code generator
/// for generating executable precondition checking functions and spec functions/methods.
pub fn parse_file_and_create_generator(file_path: &str) -> anyhow::Result<CodeGenerator> {
    let file = std::fs::read_to_string(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", file_path, e))?;
    let syntax: File = verus_syn::parse_file(&file)
        .map_err(|e| anyhow::anyhow!("Failed to parse file {}: {}", file_path, e))?;

    let (spec_fns, spec_methods) = SpecFunctionCollector::new().collect(&syntax);
    let (func_preconds, method_preconds) = PrecondCollector::new().collect(&syntax);

    Ok(CodeGenerator::new(
        spec_fns,
        spec_methods,
        func_preconds,
        method_preconds,
    ))
}

#[cfg(test)]
#[test]
fn main() {
    let generator = parse_file_and_create_generator("bitalloc16.rs").unwrap();
    let code = generator.generate_all();
    let code = prettyplease::unparse(&syn::parse2(code).unwrap());
    std::fs::write("pre.rs", code).unwrap();
}
