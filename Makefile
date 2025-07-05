install-deps:
	@# Install rustfmt and clippy if rustup is available
	@if command -v rustup >/dev/null 2>&1; then \
	rustup component add rustfmt clippy; \
	else \
	echo "rustup not found. Please install rustup from https://rustup.rs"; \
	fi

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
