use crate::check::{CheckResult, Checker, Component};

/// Identical step: if bodies are identical -> ok; if same name but different body -> undetermined.
pub struct Identical;

impl Component for Identical {
    fn name(&self) -> &str {
        "Identical"
    }

    fn is_formal(&self) -> bool {
        true
    }

    fn note(&self) -> Option<&str> {
        Some("Compare function bodies for identity")
    }

    fn run(&self, checker: &Checker) -> CheckResult {
        let mut res = CheckResult {
            status: Ok(()),
            ok: vec![],
            fail: vec![],
        };

        // only consider functions present in both srcs (unchecked sets already contain intersection)
        for func in &checker.under_checking_funcs {
            if func.body1 == func.body2 {
                res.ok.push(func.metadata.name.clone());
            }
        }

        res
    }
}
