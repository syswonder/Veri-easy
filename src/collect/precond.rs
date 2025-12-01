//! Collect preconditions using `precond-translator` crate.

use crate::defs::{Path, Precondition};
use anyhow::Result;

/// Calls the Verus precondition collector, returns the generated code and precondition list.
pub fn collect_preconds(verus_src: &str) -> Result<(String, Vec<Precondition>)> {
    // Construct the precondition generator from the Verus source code.
    let precond_gen = precond_translator::parse_file_and_create_generator(verus_src)?;

    // Generate all precondition code.
    let code = precond_gen.generate_all();
    let code = prettyplease::unparse(&syn::parse2(code).unwrap());

    // Collect function and method preconditions.
    let mut precondtions = Vec::new();
    for func in precond_gen.get_function_preconds() {
        precondtions.push(Precondition::new(Path::from_str(&func), false));
    }
    for method in precond_gen.get_method_preconds() {
        precondtions.push(Precondition::new(Path::from_str(&method), true));
    }

    Ok((code, precondtions))
}
