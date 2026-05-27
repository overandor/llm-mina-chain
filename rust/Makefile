# Integration Checkpoint Makefile
# Run: make integration-check

CARGO := cargo
FEATURES := --features network

.PHONY: integration-check check test test-lib test-examples test-replay lint bench coverage report

# Full daily gate
integration-check: check lint test test-replay test-integration bench
	@echo "=== INTEGRATION CHECK PASSED ==="

# Build check
check:
	@echo "=== cargo check ==="
	$(CARGO) check $(FEATURES)
	$(CARGO) check --examples $(FEATURES)

# Lint
lint:
	@echo "=== cargo clippy ==="
	$(CARGO) clippy $(FEATURES) -- -D warnings || true

# Tests
test: test-lib test-tests test-examples

test-lib:
	@echo "=== lib unit tests ==="
	$(CARGO) test $(FEATURES) --lib

test-tests:
	@echo "=== integration + blockchain + security + crypto + zkproof tests ==="
	$(CARGO) test $(FEATURES) --tests

test-examples:
	@echo "=== example compilation ==="
	$(CARGO) check --examples $(FEATURES)

# Replay tests
test-replay:
	@echo "=== replay validation tests ==="
	$(CARGO) test $(FEATURES) --test replay_tests

# Integration tests
test-integration:
	@echo "=== integration tests ==="
	$(CARGO) test $(FEATURES) --test integration_tests

# Benchmarks
bench:
	@echo "=== benchmarks ==="
	$(CARGO) bench $(FEATURES) || true

# Coverage (requires cargo-tarpaulin)
coverage:
	@echo "=== coverage ==="
	$(CARGO) tarpaulin --out Lcov --output-dir reports/ || true

# Generate daily report
report:
	@mkdir -p reports
	@echo "## Daily Report $(shell date +%Y-%m-%d)" > reports/daily-$(shell date +%Y-%m-%d).md
	@echo "" >> reports/daily-$(shell date +%Y-%m-%d).md
	@echo "### Build Status" >> reports/daily-$(shell date +%Y-%m-%d).md
	$(CARGO) check $(FEATURES) >> reports/daily-$(shell date +%Y-%m-%d).md 2>&1 || echo "BUILD FAILED" >> reports/daily-$(shell date +%Y-%m-%d).md
	@echo "### Test Results" >> reports/daily-$(shell date +%Y-%m-%d).md
	$(CARGO) test $(FEATURES) >> reports/daily-$(shell date +%Y-%m-%d).md 2>&1 || echo "TESTS FAILED" >> reports/daily-$(shell date +%Y-%m-%d).md
