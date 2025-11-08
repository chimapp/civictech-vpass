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

.PHONY: docker-build
docker-build:
	docker build -t vpass .

.PHONY: test
test:
	cargo test $(if $(TEST),$(TEST),)

.PHONY: coverage
coverage:
	cargo tarpaulin --out Html

.PHONY: psql
psql:
	psql "postgresql://postgres:password@localhost:5432/vpass_dev"

SQLX_CLI_STAMP := .sqlx-cli-rustls.stamp
$(SQLX_CLI_STAMP):
	cargo install sqlx-cli --locked --force --no-default-features --features "postgres,rustls"
	touch $@

.PHONY: migrate
migrate: $(SQLX_CLI_STAMP)
	cargo sqlx migrate run

.PHONY: migrate-add
migrate-add: $(SQLX_CLI_STAMP)
	@if [ -z "$(NAME)" ]; then \
		echo "Usage: make migrate-add NAME=migration_name"; \
		exit 1; \
	fi
	cargo sqlx migrate add $(NAME)

.PHONY: migrate-revert
migrate-revert: $(SQLX_CLI_STAMP)
	cargo sqlx migrate revert
