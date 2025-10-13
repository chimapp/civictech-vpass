.PHONY: dev
dev:
	cargo watch -x run

.PHONY: format
format:
	cargo fmt

.PHONY: check
check:
	cargo fmt -- --check
	cargo clippy

.PHONY: build
build:
	cargo build

.PHONY: test
test:
	cargo test $(if $(TEST),$(TEST),)

.PHONY: coverage
coverage:
	cargo tarpaulin --out Html
