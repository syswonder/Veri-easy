//! Utility functions and helpers.

use crate::log;
use anyhow::anyhow;
use std::{
    io::{BufRead, Write},
    process::{Command, ExitStatus},
};

/// Run a subprocess command and log its stderr though global logger, optionally capturing stdout to a file.
pub fn run_command(
    program: &str,
    args: &[&str],
    output_path: Option<&str>,
    work_dir: Option<&str>,
) -> anyhow::Result<ExitStatus> {
    log!(
        Verbose,
        Info,
        "Logging stderr of command '{} {}':",
        program,
        args.join(" ")
    );

    // Prepare output file if needed
    let output_file = if let Some(path) = output_path {
        Some(
            std::fs::File::create(path)
                .map_err(|e| anyhow::anyhow!("Failed to open output file: {}", e))?,
        )
    } else {
        None
    };

    // Change working directory if specified
    let cur_dir = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;
    if let Some(dir) = work_dir {
        std::env::set_current_dir(dir)
            .map_err(|e| anyhow::anyhow!("Failed to set working directory: {}", e))?;
    }

    // Spawn the command
    let mut cmd = Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn command: {}", e))?;

    // Restore original working directory
    if work_dir.is_some() {
        std::env::set_current_dir(cur_dir)
            .map_err(|e| anyhow::anyhow!("Failed to restore working directory: {}", e))?;
    }

    let stderr = cmd.stderr.take().expect("Failed to capture stderr");
    let stdout = cmd.stdout.take().expect("Failed to capture stdout");

    // Create thread to log stderr
    let log_err = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                log!(Verbose, Simple, "{}", line);
            }
        }
    });
    // Create thread to save stdout if needed
    let save_out = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        if let Some(mut file) = output_file {
            for line in reader.lines() {
                if let Ok(line) = line {
                    // log!(Verbose, Simple, "{}", line);
                    writeln!(file, "{}", line).expect("Failed to write stdout to file");
                }
            }
        } else {
            for line in reader.lines() {
                if let Ok(line) = line {
                    log!(Verbose, Simple, "{}", line);
                }
            }
        }
    });

    // Wait for command to finish and join threads
    let output = cmd
        .wait_with_output()
        .map_err(|e| anyhow::anyhow!("Failed to wait for command: {}", e))?;
    log_err
        .join()
        .expect("Failed to join stderr logging thread");
    save_out
        .join()
        .expect("Failed to join stdout saving thread");

    // Treat Kani's exit code 1 (unsure verification) as normal.
    let is_kani_exit_1 = program == "cargo"
    && args.iter().any(|a| *a == "kani")
    && output.status.code() == Some(1);

    if output.status.success() || is_kani_exit_1 {
        log!(
            Verbose,
            Info,
            "Command '{}' finished successfully.",
            program
        );
    } else {
        log!(
            Normal,
            Error,
            "Command '{}' failed with exit code: {}",
            program,
            output.status
        );
    }
    Ok(output.status)
}

/// Create a typical harness project directory structure. Dir structure:
///
/// harness_path
/// ├── Cargo.toml
/// └── src
///     ├── main.rs
///     ├── mod1.rs
///     └── mod2.rs
pub fn create_harness_project(
    path: &str,
    src1: &str,
    src2: &str,
    harness: &str,
    toml: &str,
    lib: bool,
) -> anyhow::Result<()> {
    // Remove existing directory if any
    if std::path::Path::new(path).exists() {
        std::fs::remove_dir_all(path)
            .map_err(|_| anyhow!("Failed to remove existing harness directory"))?;
    }
    let project_type = if lib { "--lib" } else { "--bin" };
    run_command(
        "cargo",
        &["new", project_type, "--vcs", "none", path],
        None,
        None,
    )?;
    let harness_file = path.to_owned() + if lib { "/src/lib.rs" } else { "/src/main.rs" };

    // Write rust files
    std::fs::File::create(path.to_owned() + "/src/mod1.rs")
        .unwrap()
        .write_all(src1.as_bytes())
        .map_err(|_| anyhow!("Failed to write mod1 file"))?;
    std::fs::File::create(path.to_owned() + "/src/mod2.rs")
        .unwrap()
        .write_all(src2.as_bytes())
        .map_err(|_| anyhow!("Failed to write mod2 file"))?;
    std::fs::File::create(harness_file)
        .unwrap()
        .write_all(harness.as_bytes())
        .map_err(|_| anyhow!("Failed to write harness file"))?;

    // Write Cargo.toml
    std::fs::File::create(path.to_owned() + "/Cargo.toml")
        .unwrap()
        .write_all(toml.as_bytes())
        .map_err(|_| anyhow!("Failed to write Cargo.toml"))?;

    // Cargo fmt
    let cur_dir = std::env::current_dir().unwrap();
    let _ = std::env::set_current_dir(path);
    run_command("cargo", &["fmt"], None, None)?;
    let _ = std::env::set_current_dir(cur_dir);

    Ok(())
}
