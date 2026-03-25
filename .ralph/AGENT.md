# Agent Instructions

## Build
```bash
cargo build
```

## Check
```bash
cargo check
```

## Test
```bash
cargo test
```

## Run
```bash
cargo run -p codeforge-app
```

## Lint
```bash
cargo clippy -- -D warnings
```

## Format
```bash
cargo fmt --check
```

## Equipped Tools
- Context7: Use `mcp__plugin_context7_context7__resolve-library-id` then `query-docs` for any library docs
- CoVe: Apply 4-stage verification on non-trivial code (see PROMPT.md §4)

## GSD Commands
- Execute plan: Read the PLAN.md and follow its tasks
- Write summary: Create SUMMARY.md after plan completion
- Update state: Modify STATE.md with progress

## Project Reference
- t3code source: /tmp/t3code (reference implementation)
- Codex protocol: /tmp/t3code/apps/server/src/codexAppServerManager.ts
- Claude adapter: /tmp/t3code/apps/server/src/provider/Layers/ClaudeAdapter.ts
- Provider events: /tmp/t3code/packages/contracts/src/providerRuntime.ts
