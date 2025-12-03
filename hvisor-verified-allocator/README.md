# Formal Verification of Memory Allocator in Hvisor

This project provides three implementations and a verified model of the BitAlloc memory allocator:

* **original** — the original implementation
* **verified_impl** — the Verus-verified implementation
* **verified_proof** — the Verus-verified model
* **optimized** — the optimized implementation rebuilt based on performance analysis and integration constrains


## Usage Guide

## 1. Environment

You need:

* Rust toolchain (`rustup`)
* `cargo` (comes with Rust)
* Verus (for verification) Ensure `Verus` is installed and usable in the environment.
  See [Verus docs](https://github.com/verus-lang/verus/blob/main/INSTALL.md) for installation instructions

Verus must be available in `$PATH` as `verus`.

---

## Commands

This project supports **two commands**:

---

## Performance evaluation

* Comparing performance of `original` / `verified_impl` / `optimized`
* Generating benchmark reports

### Command:

```bash
make bench
```

---

## Verification

* Used for running Verus proofs.

### Command:

```bash
make verify
```

---

# ❗ Notes

* The proof module depends on Verus; make sure `verus` is installed before running `make verify`.
* Always run `make bench` / `make verify` instead of manually editing `lib.rs`.

---
