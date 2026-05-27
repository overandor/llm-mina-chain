#!/usr/bin/env bash
set -euo pipefail

# Daily Integration Checkpoint Script
# Run this every day before merging feature branches.
#
# Usage:
#   chmod +x integration-check.sh
#   ./integration-check.sh

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASS=0
FAIL=0

function banner() {
    echo ""
    echo "========================================"
    echo "  $1"
    echo "========================================"
}

function ok() {
    echo -e "${GREEN}PASS${NC}: $1"
    PASS=$((PASS + 1))
}

function err() {
    echo -e "${RED}FAIL${NC}: $1"
    FAIL=$((FAIL + 1))
}

function warn() {
    echo -e "${YELLOW}WARN${NC}: $1"
}

cd "$(dirname "$0")/rust"

banner "1. LIBRARY COMPILATION"
if cargo check --lib 2>&1 | grep -q "error\[E"; then
    err "Library compilation failed"
else
    ok "Library compiles cleanly"
fi

banner "2. ALL BINARIES BUILD"
BINS=("llm-mina-node" "solana-agent-server" "solana-agent-cli" "semantic-runtime")
for bin in "${BINS[@]}"; do
    if cargo build --bin "$bin" 2>&1 | grep -q "error\[E"; then
        err "Binary $bin failed to build"
    else
        ok "Binary $bin builds"
    fi
done

banner "3. UNIT TESTS (lib)"
if cargo test --lib 2>&1 | grep -q "test result: FAILED"; then
    err "Unit tests failed"
else
    ok "Unit tests pass"
fi

banner "4. INTEGRATION TESTS"
if cargo test --test daily_integration 2>&1 | grep -q "test result: FAILED"; then
    err "Integration tests failed"
else
    ok "Integration tests pass"
fi

banner "5. EXISTING INTEGRATION TESTS"
if cargo test --test integration_tests 2>&1 | grep -q "test result: FAILED"; then
    err "Existing integration tests failed"
else
    ok "Existing integration tests pass"
fi

banner "6. BENCHMARKS COMPILE"
if cargo build --bench blockchain_bench 2>&1 | grep -q "error\[E"; then
    err "Benchmarks failed to compile"
else
    ok "Benchmarks compile"
fi

banner "7. EXAMPLES COMPILE"
if cargo build --examples 2>&1 | grep -q "error\[E"; then
    err "Examples failed to compile"
else
    ok "Examples compile"
fi

banner "8. NETWORK FEATURE COMPILATION"
if cargo check --features network --lib 2>&1 | grep -q "error\[E"; then
    err "Network feature compilation failed"
else
    ok "Network feature compiles"
fi

banner "9. PROTOCOL MODULE TESTS"
if cargo test protocol 2>&1 | grep -q "test result: FAILED"; then
    err "Protocol module tests failed"
else
    ok "Protocol module tests pass"
fi

banner "10. SOLANA AGENT TESTS"
if cargo test solana_agent 2>&1 | grep -q "test result: FAILED"; then
    err "Solana agent tests failed"
else
    ok "Solana agent tests pass"
fi

banner "SUMMARY"
TOTAL=$((PASS + FAIL))
echo ""
echo "Total checks: $TOTAL"
echo -e "Passed: ${GREEN}$PASS${NC}"
echo -e "Failed: ${RED}$FAIL${NC}"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}INTEGRATION CHECK FAILED${NC}"
    echo "Stop feature work. Fix stability first."
    exit 1
else
    echo -e "${GREEN}INTEGRATION CHECK PASSED${NC}"
    echo "Safe to merge."
    exit 0
fi
