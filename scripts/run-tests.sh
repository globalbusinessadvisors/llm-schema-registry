#!/bin/bash
set -euo pipefail

# Comprehensive test runner for LLM Schema Registry

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse arguments
TEST_TYPE="${1:-all}"
COVERAGE="${2:-false}"

log_info "Starting test suite: $TEST_TYPE"

# Clean previous test artifacts
clean_tests() {
    log_info "Cleaning previous test artifacts..."
    cargo clean
    rm -rf target/coverage
    mkdir -p target/coverage
}

# Run unit tests
run_unit_tests() {
    log_info "Running unit tests..."
    cargo test --lib --bins --all-features --no-fail-fast -- --nocapture

    if [ $? -eq 0 ]; then
        log_info "Unit tests passed!"
    else
        log_error "Unit tests failed!"
        return 1
    fi
}

# Run integration tests
run_integration_tests() {
    log_info "Running integration tests (requires Docker)..."

    # Check if Docker is running
    if ! docker info > /dev/null 2>&1; then
        log_error "Docker is not running. Integration tests require Docker."
        return 1
    fi

    cargo test --test integration --all-features -- --nocapture

    if [ $? -eq 0 ]; then
        log_info "Integration tests passed!"
    else
        log_error "Integration tests failed!"
        return 1
    fi
}

# Run E2E tests
run_e2e_tests() {
    log_info "Running end-to-end tests..."
    cargo test --test e2e --all-features -- --nocapture

    if [ $? -eq 0 ]; then
        log_info "E2E tests passed!"
    else
        log_error "E2E tests failed!"
        return 1
    fi
}

# Run property-based tests
run_property_tests() {
    log_info "Running property-based tests..."
    cargo test --test property --all-features -- --nocapture

    if [ $? -eq 0 ]; then
        log_info "Property tests passed!"
    else
        log_error "Property tests failed!"
        return 1
    fi
}

# Run benchmarks
run_benchmarks() {
    log_info "Running benchmarks..."
    cargo bench --all-features

    if [ $? -eq 0 ]; then
        log_info "Benchmarks completed!"
    else
        log_warn "Benchmarks failed (non-critical)"
    fi
}

# Run code coverage
run_coverage() {
    log_info "Running code coverage with tarpaulin..."

    # Install tarpaulin if not present
    if ! command -v cargo-tarpaulin &> /dev/null; then
        log_info "Installing cargo-tarpaulin..."
        cargo install cargo-tarpaulin
    fi

    cargo tarpaulin --config tarpaulin.toml --engine llvm

    if [ $? -eq 0 ]; then
        log_info "Coverage report generated at target/coverage/"
        log_info "Open target/coverage/index.html in browser to view"
    else
        log_error "Coverage generation failed!"
        return 1
    fi
}

# Run linting
run_lint() {
    log_info "Running linting..."

    # Format check
    log_info "Checking code formatting..."
    cargo fmt --all -- --check

    # Clippy
    log_info "Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings

    if [ $? -eq 0 ]; then
        log_info "Linting passed!"
    else
        log_error "Linting failed!"
        return 1
    fi
}

# Run security audit
run_security_audit() {
    log_info "Running security audit..."

    # Install cargo-audit if not present
    if ! command -v cargo-audit &> /dev/null; then
        log_info "Installing cargo-audit..."
        cargo install cargo-audit
    fi

    cargo audit

    if [ $? -eq 0 ]; then
        log_info "Security audit passed!"
    else
        log_warn "Security audit found issues (check output)"
    fi
}

# Main execution
main() {
    case "$TEST_TYPE" in
        unit)
            run_unit_tests
            ;;
        integration)
            run_integration_tests
            ;;
        e2e)
            run_e2e_tests
            ;;
        property)
            run_property_tests
            ;;
        bench)
            run_benchmarks
            ;;
        lint)
            run_lint
            ;;
        security)
            run_security_audit
            ;;
        coverage)
            run_coverage
            ;;
        all)
            clean_tests
            run_lint || exit 1
            run_security_audit
            run_unit_tests || exit 1
            run_integration_tests || exit 1
            run_e2e_tests || exit 1
            run_property_tests || exit 1

            if [ "$COVERAGE" = "true" ]; then
                run_coverage || exit 1
            fi

            log_info "All tests passed! ✓"
            ;;
        *)
            log_error "Unknown test type: $TEST_TYPE"
            echo "Usage: $0 {unit|integration|e2e|property|bench|lint|security|coverage|all} [coverage]"
            exit 1
            ;;
    esac
}

main
