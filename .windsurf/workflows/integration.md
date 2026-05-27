---
description: Daily Integration Checkpoint for all three agents
---
# Daily Integration Checkpoint

Run this every day before any feature work. If it fails, stop and fix.

## Prerequisites

All three agents must have pushed to the integration branch.

## Steps

1. Merge all agent branches into `integration`:
   ```bash
   git checkout integration
   git merge agent-1-core
   git merge agent-2-query-api
   git merge agent-3-proof-provenance
   ```

2. Run the full integration gate:
   ```bash
   make integration-check
   ```

3. Verify no drift in canonical artifacts:
   - `rust/SCHEMA.md` must be unchanged
   - `rust/ARCHITECTURE.md` must be unchanged
   - `rust/Canonical.toml` must be unchanged

4. If any test fails, stop all feature work and file an integration ticket.

## Output

The checkpoint produces:
- `reports/daily-YYYY-MM-DD.md` — test results, build status, drift report
- `reports/coverage-YYYY-MM-DD.lcov` — coverage snapshot
- `reports/bench-YYYY-MM-DD.txt` — benchmark snapshot

## Rules

- No agent may introduce a new dependency without approval from Agent 1 (Core Runtime).
- No agent may change `SCHEMA.md` without a three-agent review.
- Mock cryptography is forbidden in all integration branches.
- Placeholder responses are forbidden in all integration branches.
- Silent fallback behavior must be logged at `WARN` level minimum.
