.PHONY: help build test check fmt lint clean run-server run-cli doc install

# Default target
help:
	@echo "LLM Schema Registry - Makefile Commands"
	@echo "========================================"
	@echo "  make build       - Build all crates"
	@echo "  make test        - Run all tests"
	@echo "  make check       - Check code (fast compile check)"
	@echo "  make fmt         - Format code with rustfmt"
	@echo "  make lint        - Run clippy linter"
	@echo "  make clean       - Clean build artifacts"
	@echo "  make run-server  - Run the schema registry server"
	@echo "  make run-cli     - Run the CLI tool"
	@echo "  make doc         - Generate documentation"
	@echo "  make install     - Install binaries"

# Build all crates
build:
	cargo build --workspace

# Build release version
build-release:
	cargo build --workspace --release

# Run all tests
test:
	cargo test --workspace

# Quick compile check
check:
	cargo check --workspace

# Format code
fmt:
	cargo fmt --all

# Check formatting
fmt-check:
	cargo fmt --all -- --check

# Run clippy linter
lint:
	cargo clippy --workspace -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Run the server
run-server:
	cargo run --bin schema-registry-server

# Run the CLI
run-cli:
	cargo run --bin schema-cli -- --help

# Generate documentation
doc:
	cargo doc --workspace --no-deps --open

# Generate documentation without opening
doc-build:
	cargo doc --workspace --no-deps

# Install binaries
install:
	cargo install --path crates/schema-registry-server
	cargo install --path crates/schema-registry-cli

# Run tests with coverage (requires cargo-tarpaulin)
test-coverage:
	cargo tarpaulin --workspace --out Html --output-dir coverage

# Run benchmarks
bench:
	cargo bench --workspace

# Watch for changes and rebuild (requires cargo-watch)
watch:
	cargo watch -x check -x test

# Development setup
dev-setup:
	rustup component add rustfmt clippy
	cargo install cargo-watch cargo-tarpaulin

# Database migrations (placeholder)
db-migrate:
	@echo "Database migrations not yet implemented"

# Docker build
docker-build:
	docker build -t schema-registry:latest .

# Docker compose up
docker-up:
	docker-compose up -d

# Docker compose down
docker-down:
	docker-compose down

# All checks (fmt, lint, test)
ci: fmt-check lint test
	@echo "All CI checks passed!"

# ============================================================================
# Deployment Commands
# ============================================================================

# Docker operations
.PHONY: docker-build docker-push docker-up docker-down docker-logs

docker-build:
	docker build -t schema-registry:latest .

docker-build-cli:
	docker build --target runtime-cli -t schema-registry-cli:latest .

docker-push:
	docker tag schema-registry:latest ghcr.io/llm-schema-registry/schema-registry:latest
	docker push ghcr.io/llm-schema-registry/schema-registry:latest

docker-up:
	docker-compose up -d

docker-down:
	docker-compose down

docker-logs:
	docker-compose logs -f schema-registry

docker-clean:
	docker-compose down -v

# Kubernetes operations
.PHONY: k8s-deploy k8s-delete k8s-status k8s-logs k8s-port-forward

k8s-deploy:
	kubectl apply -k deployments/kubernetes/base/

k8s-delete:
	kubectl delete -k deployments/kubernetes/base/

k8s-status:
	kubectl get pods -n schema-registry
	kubectl get svc -n schema-registry

k8s-logs:
	kubectl logs -f deployment/schema-registry -n schema-registry

k8s-port-forward:
	kubectl port-forward -n schema-registry svc/schema-registry-service 8080:80

# Helm operations
.PHONY: helm-install helm-upgrade helm-uninstall helm-status helm-lint

helm-install:
	helm install schema-registry ./helm/schema-registry \
		--namespace schema-registry \
		--create-namespace

helm-upgrade:
	helm upgrade schema-registry ./helm/schema-registry \
		--namespace schema-registry \
		--reuse-values

helm-uninstall:
	helm uninstall schema-registry -n schema-registry

helm-status:
	helm status schema-registry -n schema-registry

helm-lint:
	helm lint ./helm/schema-registry

helm-template:
	helm template schema-registry ./helm/schema-registry

# Monitoring
.PHONY: monitoring-up monitoring-down

monitoring-up:
	docker-compose --profile monitoring up -d

monitoring-down:
	docker-compose --profile monitoring down
