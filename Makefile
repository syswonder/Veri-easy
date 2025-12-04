# Path to lib.rs of the allocator crate (from workspace root)
LIB_RS := hvisor-verified-allocator/src/lib.rs
OS := $(shell uname)

ifeq ($(OS),Darwin)
  SED_INPLACE = sed -i ''     # macOS / BSD sed
else
  SED_INPLACE = sed -i        # Linux / GNU sed
endif

# Comment out verified_proof: used for performance testing / cargo bench
comment_proof:
	$(SED_INPLACE) 's/^pub mod verified_proof;/\/\/ pub mod verified_proof;/' $(LIB_RS)

# Uncomment verified_proof: used for running Verus verification
uncomment_proof:
	$(SED_INPLACE) 's/^\/\/[[:space:]]*pub mod verified_proof;/pub mod verified_proof;/' $(LIB_RS)

# Benchmark the allocator crate only
bench: comment_proof
	cargo bench --manifest-path hvisor-verified-allocator/Cargo.toml

# Run Verus on the proof module
verify: uncomment_proof
	verus hvisor-verified-allocator/src/verified_proof.rs

# Run the Veri-easy workflow on the original and verified implementations of the allocator
verieasy: 
	cargo run -- -p hvisor-verified-allocator/src/verified_proof.rs \
	hvisor-verified-allocator/src/original.rs hvisor-verified-allocator/src/verified_impl.rs -l normal
