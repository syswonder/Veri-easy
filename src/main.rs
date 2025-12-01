use clap::Parser;

use crate::{
    check::{Checker, Source},
    collect::collect_preconds,
    config::{VerieasyConfig, WorkflowConfig},
};

mod check;
mod collect;
mod components;
mod config;
mod defs;
mod generate;
mod log;
mod utils;

fn main() {
    // Parse global configuration
    let config = VerieasyConfig::parse();

    // Initialize logger
    log::init_logger(config.log);
    log!(
        Brief,
        Critical,
        "Veri-easy version {}",
        env!("CARGO_PKG_VERSION")
    );
    log!(Brief, Info, "Log level set to {:?}", config.log);

    // Load workflow configuration
    let res = WorkflowConfig::parse(&config.config);
    if let Err(e) = &res {
        log!(
            Brief,
            Error,
            "Failed to parse workflow configuration: {}",
            e
        );
        return;
    }
    let workflow_config = res.unwrap();
    log!(Brief, Simple, "");
    workflow_config.log();

    // Construct workflow components
    let components = workflow_config.construct_workflow();

    // Load source files
    let res = Source::open(&config.file1);
    if let Err(e) = &res {
        log!(
            Brief,
            Error,
            "Failed to open source file {}: {}",
            &config.file1,
            e
        );
        return;
    }
    let s1 = res.unwrap();
    let res = Source::open(&config.file2);
    if let Err(e) = &res {
        log!(
            Brief,
            Error,
            "Failed to open source file {}: {}",
            &config.file2,
            e
        );
        return;
    }
    let mut s2 = res.unwrap();

    // Collect preconditions
    let (precond_code, preconditions) = if let Some(precond_path) = &config.preconditions {
        match collect_preconds(precond_path) {
            Ok((code, preconditions)) => (code, preconditions),
            Err(e) => {
                log!(
                    Brief,
                    Error,
                    "Failed to collect preconditions from {}: {}",
                    precond_path,
                    e
                );
                (String::new(), Vec::new())
            }
        }
    } else {
        (String::new(), Vec::new())
    };
    // Append preconditions to source 2
    s2.append_content(&precond_code);

    log!(Brief, Simple, "");
    log!(
        Brief,
        Critical,
        "Starting verification between `{}` and `{}`\n",
        s1.path,
        s2.path
    );

    // Create checker and run workflow
    let mut checker = Checker::new(s1, s2, components, preconditions, config.strict);
    log!(Normal, Info, "Logging initial state:");
    checker.print_state();
    log!(Normal, Simple, "");

    checker.run_all();
}
