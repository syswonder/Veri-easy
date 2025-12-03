LIB_RS := src/lib.rs
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

bench: comment_proof
	cargo bench

verify: uncomment_proof
	verus src/verified_proof.rs
