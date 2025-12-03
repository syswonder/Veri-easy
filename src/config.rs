//! Configuration Veri-easy workflow and components.
use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{check::Component, components::*, log, log::LogLevel};

/// Veri-easy Functional Equivalence Checker.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct VerieasyConfig {
    /// Path to the workflow configuration file.
    #[clap(short, long, default_value = "workflow.toml")]
    pub config: String,
    /// Log level.
    #[clap(short, long, default_value = "normal")]
    #[arg(value_enum)]
    pub log: LogLevel,
    /// File from which to collect preconditions.
    #[clap(short = 'p', long)]
    pub preconditions: Option<String>,
    /// Strict mode: exit on first error.
    #[clap(short = 's', long, default_value_t = false)]
    pub strict: bool,
    /// Source file 1, usually the original source.
    pub file1: String,
    /// Source file 2, usually the Verus refactored source.
    pub file2: String,
}

/// Configuration for Kani component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KaniConfig {
    /// Kani harness path.
    pub harness_path: String,
    /// Kani output path.
    pub output_path: String,
    /// Timeout in seconds for Kani execution.
    pub timeout_secs: u64,
    /// Whether to generate new harness.
    pub gen_harness: bool,
    /// Keep intermediate harness project.
    pub keep_harness: bool,
    /// Keep Kani output file.
    pub keep_output: bool,
    /// Use preconditions.
    pub use_preconditions: bool,
    /// Loop unwind bound.
    pub loop_unwind: Option<u32>,
}

impl Default for KaniConfig {
    fn default() -> Self {
        KaniConfig {
            harness_path: "kani_harness".to_string(),
            output_path: "kani.tmp".to_string(),
            timeout_secs: 300,
            gen_harness: true,
            keep_harness: false,
            keep_output: false,
            use_preconditions: true,
            loop_unwind: None,
        }
    }
}

/// Configuration for Alive2 component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Alive2Config {
    /// Path to Alive2 binary.
    pub alive2_path: String,
    /// Alive2 output path.
    pub output_path: String,
    /// Keep Alive2 output file.
    pub keep_output: bool,
}

impl Default for Alive2Config {
    fn default() -> Self {
        Alive2Config {
            alive2_path: "alive2-tv".to_string(),
            output_path: "alive2.tmp".to_string(),
            keep_output: false,
        }
    }
}

/// Configuration for Differential Fuzzing component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiffFuzzConfig {
    /// Fuzzing harness path.
    pub harness_path: String,
    /// Fuzzing output path.
    pub output_path: String,
    /// Executions for fuzzing.
    pub executions: u32,
    /// Initial input count.
    pub initial_inputs: usize,
    /// Length of each input.
    pub input_len: usize,
    /// Keep fuzzing harness project.
    pub keep_harness: bool,
    /// Keep fuzzing output file.
    pub keep_output: bool,
    /// Use preconditions.
    pub use_preconditions: bool,
    /// Catch panic unwind.
    pub catch_panic: bool,
    /// Enable log in fuzzing harness
    pub harness_log: bool,
}

impl Default for DiffFuzzConfig {
    fn default() -> Self {
        DiffFuzzConfig {
            harness_path: "df_harness".to_string(),
            output_path: "df.tmp".to_string(),
            executions: 1000,
            initial_inputs: 16,
            input_len: 131072,
            keep_harness: false,
            keep_output: false,
            use_preconditions: true,
            catch_panic: true,
            harness_log: true,
        }
    }
}

/// Configuration for Property-Based Testing component.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PBTConfig {
    /// PBT harness path.
    pub harness_path: String,
    /// PBT output path.
    pub output_path: String,
    /// Test cases.
    pub test_cases: usize,
    /// Keep PBT harness project.
    pub keep_harness: bool,
    /// Keep PBT output file.
    pub keep_output: bool,
    /// Use preconditions.
    pub use_preconditions: bool,
}

impl Default for PBTConfig {
    fn default() -> Self {
        PBTConfig {
            harness_path: "pbt_harness".to_string(),
            output_path: "pbt.tmp".to_string(),
            test_cases: 10000,
            keep_harness: false,
            keep_output: false,
            use_preconditions: true,
        }
    }
}

/// Workflow configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowConfig {
    /// Workflow.
    pub components: Vec<String>,
    /// Kani component configuration.
    pub kani: Option<KaniConfig>,
    /// Alive2 component configuration.
    pub alive2: Option<Alive2Config>,
    /// Differential Fuzzing component configuration.
    pub diff_fuzz: Option<DiffFuzzConfig>,
    /// Property-Based Testing component configuration.
    pub pbt: Option<PBTConfig>,
}

impl WorkflowConfig {
    /// Parse workflow configuration from a TOML file.
    pub fn parse(config_file: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(config_file)
            .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
        let mut config: WorkflowConfig = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
        // Check components and fill in default configurations for missing components.
        let msg = |comp: &str| {
            format!(
                "Component `{}` is selected in workflow but no configuration found. Using default configuration.",
                comp
            )
        };
        for component in &config.components {
            match component.to_lowercase().as_str() {
                "identical" => (),
                "kani" => {
                    if config.kani.is_none() {
                        log!(Brief, Warning, &msg("Kani"));
                        config.kani = Some(KaniConfig::default());
                    }
                }
                "pbt" => {
                    if config.pbt.is_none() {
                        log!(Brief, Warning, &msg("PBT"));
                        config.pbt = Some(PBTConfig::default());
                    }
                }
                "difffuzz" | "diff-fuzz" | "diff_fuzz" => {
                    if config.diff_fuzz.is_none() {
                        log!(Brief, Warning, &msg("Differential Fuzzing"));
                        config.diff_fuzz = Some(DiffFuzzConfig::default());
                    }
                }
                "alive2" => {
                    if config.alive2.is_none() {
                        log!(Brief, Warning, &msg("Alive2"));
                        config.alive2 = Some(Alive2Config::default());
                    }
                }
                other => {
                    log!(
                        Brief,
                        Warning,
                        "Unknown component `{}` in configuration. Ignoring.",
                        other
                    );
                }
            }
        }
        Ok(config)
    }

    /// Log the loaded workflow configuration.
    pub fn log(&self) {
        log!(
            Brief,
            Critical,
            "Workflow: {}",
            self.components.join(" -> ")
        );
        if let Some(kani_cfg) = &self.kani {
            log!(Normal, Info, "Kani Config: {:?}", kani_cfg);
        }
        if let Some(alive2_cfg) = &self.alive2 {
            log!(Normal, Info, "Alive2 Config: {:?}", alive2_cfg);
        }
        if let Some(diff_fuzz_cfg) = &self.diff_fuzz {
            log!(
                Normal,
                Info,
                "Differential Fuzzing Config: {:?}",
                diff_fuzz_cfg
            );
        }
        if let Some(pbt_cfg) = &self.pbt {
            log!(Normal, Info, "Property-Based Testing Config: {:?}", pbt_cfg);
        }
    }

    /// Construct workflow components based on the configuration.
    pub fn construct_workflow(&self) -> Vec<Box<dyn Component>> {
        let mut components: Vec<Box<dyn Component>> = Vec::new();
        for component in &self.components {
            match component.to_lowercase().as_str() {
                "identical" => components.push(Box::new(Identical)),
                "kani" => components.push(Box::new(Kani::new(self.kani.to_owned().unwrap()))),
                "pbt" => components.push(Box::new(PropertyBasedTesting::new(
                    self.pbt.to_owned().unwrap(),
                ))),
                "difffuzz" | "diff-fuzz" | "diff_fuzz" => components.push(Box::new(
                    DifferentialFuzzing::new(self.diff_fuzz.to_owned().unwrap()),
                )),
                "alive2" => components.push(Box::new(Alive2::new(self.alive2.to_owned().unwrap()))),
                other => log!(
                    Brief,
                    Warning,
                    "Unknown component `{}` in configuration. Ignoring.",
                    other
                ),
            }
        }
        components
    }
}
