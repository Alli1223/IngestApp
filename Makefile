install-deps:
	rustup component add rustfmt clippy

fmt:
	cargo fmt

check:
	cargo check

test:
	cargo test

run:
	cargo run

build:
	cargo build --release

.PHONY: install-deps fmt check test run build
