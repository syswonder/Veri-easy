# Veri-easy

Veri-easy is a lightweight and automated framework that combines multiple formal and testing techniques to establish functional equivalence between the verified and original implementations. It automates function collection, harness generation, integrates with Kani model checking, property-based testing (Proptest), and differential fuzzing, and can optionally invoke Alive2 for IR-level validation.

## Features
- Functional equivalence checking across multiple components: `identical`, `kani`, `pbt`, `difffuzz`, `alive2`, and more ...
- Automatic harness generation for Kani, Proptest, and DiffFuzz with support for preconditions.
- Configurable workflow via `workflow.toml`, including component-specific knobs.
- Verus precondition/spec translator (in `precond-translator/`) to turn Verus specs into executable Rust precondition checkers.
- Clear logging with levels: `brief`, `normal`, `verbose`.

## Project Structure
- `src/main.rs`: Entry point; loads `workflow.toml`, parses CLI, orchestrates components.
- `src/config.rs`: Workflow and component configs (Kani/PBT/DiffFuzz/Alive2) and CLI schema.
- `src/check.rs`: Core checker, source parsing, workflow execution, result aggregation.
- `src/generate.rs`: Harness generator backends and helpers for functions/methods and preconditions.
- `src/components/`: Implementations of each component (`kani.rs`, `pbt.rs`, `df.rs`, `alive2.rs`, `identical.rs`).
- `src/collect/` and `src/defs/`: Function/type/path abstractions and collection utilities.
- `precond-translator/`: Verus parser and code generator for preconditions and spec functions.
- `workflow.toml`: Configures the component pipeline and per-component settings.

## Environment
- Rust toolchain and `cargo`.
- Kani (optional; required when using the `kani` component). Ensure `kani` is installed and usable in the environment.
- Proptest and `proptest-derive` are used via the PBT harness project; `cargo` handles dependencies.
- Differential fuzzing harness uses AFL/LibFuzzer style workflows; ensure local toolchain supports building and running the included harness.
- Alive2 (optional; required when `alive2` is enabled): set `alive2_path` to your `alive-tv` binary in `workflow.toml`.

## Usage
Build and run from the workspace root.

```zsh
# Build
cargo build

# Run with defaults (uses workflow.toml)
cargo run -- file1.rs file2.rs

# Specify preconditions (Verus file) and strict mode
cargo run -- -p verus_specs.rs -s file1.rs file2.rs

# Adjust log level (brief|normal|verbose)
cargo run -- -l verbose file1.rs file2.rs

# Use a different workflow config
cargo run -- -c path/to/workflow.toml file1.rs file2.rs
```

### CLI Options
- `-c, --config <FILE>`: workflow TOML (default `workflow.toml`).
- `-l, --log <LEVEL>`: `brief`, `normal`, or `verbose`.
- `-p, --preconditions <FILE>`: Verus spec file; translated and appended to `file2`.
- `-s, --strict`: exit on first error.
- Positional: `file1` and `file2` Rust source files.

### Workflow Configuration (`workflow.toml`)
Example (defaults present in repo):

```toml
components = ["kani", "pbt", "difffuzz"]

[kani]
harness_path = "kani_harness"
output_path = "kani.tmp"
timeout_secs = 1
gen_harness = true
keep_harness = true
keep_output = true
use_preconditions = true
loop_unwind = 20

[diff_fuzz]
harness_path = "df_harness"
output_path = "df.tmp"
executions = 500000
keep_harness = true
keep_output = true
catch_panic = true
use_preconditions = false

[pbt]
harness_path = "pbt_harness"
output_path = "pbt.tmp"
test_cases = 50000
keep_harness = true
keep_output = true
use_preconditions = false
```

Notes:
- Component names accepted: `identical`, `kani`, `pbt`, `difffuzz` (`diff-fuzz`, `diff_fuzz` also accepted), `alive2`.
- Missing per-component sections are filled with sensible defaults.
- `preconditions` (CLI) enables argument assumptions in harnesses when supported.
- Detailed arguments can be found in `src/config.rs`.

## How It Works
- Sources are parsed (`syn`), functions and types collected.
- (Optional) Preconditions are collected from Verus specs via the precondition translator.
- Functions/methods are matched between the two sources based on name and signature.
- For each component in the workflow, Veri-easy generates harness code and runs the tool:
	- `kani`: generates `#[kani::proof]` harnesses, supports `loop_unwind` and `Arbitrary` args.
	- `pbt`: generates Proptest tests with `prop_assume!` for preconditions and mismatch reporting.
	- `difffuzz`: generates fuzz harness; optionally catches panic and can use preconditions.
	- `alive2`: invokes `alive-tv` with configured path.
- Results are logged; strict mode stops on first fatal error.

## Requirements for Types/Methods
- Free functions vs methods are classified automatically.
- Receiver methods require a constructor and optional getter per type.
- Implement constructors named `verieasy_new` and getters `verieasy_get` in `impl` blocks to enable method harnessing.

## Contributing
Issues and PRs are welcome. Ensure changes keep harness generation minimal and respect existing component interfaces.

