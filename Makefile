CARGO = cargo

all: Cargo.toml
	$(CARGO) build --release

check: Cargo.toml
	$(CARGO) test -- --nocapture

clean: Cargo.toml
	$(CARGO) clean
	@rm -rf *~ target

.PHONY: all check clean
.SECONDARY:
.SUFFIXES:
