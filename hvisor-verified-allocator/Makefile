LIB_RS := src/lib.rs

# Comment out verified_proof: used for performance testing / cargo bench
comment_proof:
	sed -i 's/^pub mod verified_proof;/\/\/ pub mod verified_proof;/' $(LIB_RS)

# Uncomment verified_proof: used for running Verus verification
uncomment_proof:
	sed -i 's/^\/\/[[:space:]]*pub mod verified_proof;/pub mod verified_proof;/' $(LIB_RS)

# One-shot: switch to "performance mode" and run all benchmarks
bench: comment_proof
	cargo bench

# One-shot: switch to "verification mode" and run Verus on the proof module
verify: uncomment_proof
	verus src/verified_proof.rs
