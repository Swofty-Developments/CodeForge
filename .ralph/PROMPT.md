# Project: CodeForge

## Your Mission

You are working autonomously in a Ralph loop executing GSD plans. Each iteration:

### 1. Read current state
- Read `.planning/STATE.md` to find current phase and plan
- Read `.planning/ROADMAP.md` for phase context and dependencies

### 2. Determine next action

Follow this decision tree:

```
Is current phase's DISCOVERY.md missing?
  YES → Run research: read PRD, explore codebase, write DISCOVERY.md
        Use Context7 to look up docs for any unfamiliar tech.
  NO  ↓

Are there ungenerated plans for current phase?
  YES → Generate next {NN}-{NN}-PLAN.md from DISCOVERY.md + PRD
  NO  ↓

Is there an unexecuted plan in current phase?
  YES → Execute it (see "Execute a Plan" below)
  NO  ↓

Are all plans in current phase complete?
  YES → Complete phase: update ROADMAP.md, advance STATE.md to next phase
  NO  → Something is wrong. Set STATUS: BLOCKED.

Are all phases complete?
  YES → Run verification gate. If passing, set EXIT_SIGNAL: true
  NO  → Continue to next phase (loop back to top)
```

### 3. Execute a Plan

When executing a `{NN}-{NN}-PLAN.md`:

1. Read the plan file completely
2. Execute each `<task>` in order
3. **For non-trivial tasks**, apply CoVe:
   - Generate code [UNVERIFIED]
   - Plan verification targets specific to THIS code
   - Independently verify each target
   - Apply fixes → [VERIFIED] code
4. After each task, run its `<verify>` checks
5. After all tasks, run the plan's `<verification>` section
6. Write `{NN}-{NN}-SUMMARY.md` with results
7. Commit changes with descriptive message
8. Update `.planning/STATE.md` (increment plan, update metrics)

### 4. CoVe triggers

Apply the full 4-stage CoVe protocol for:
- Async/concurrent logic (tokio tasks, channels, subprocess IO)
- Database operations (rusqlite queries, migrations)
- JSON-RPC protocol handling
- Session lifecycle management
- Any code where the bug would be subtle, not obvious

Skip CoVe for: trivial one-liners, config files, pure UI layout.

### 5. Generate plans for next phase (just-in-time)

When advancing to a new phase:
1. Read DISCOVERY.md for that phase (or create it first)
2. Read relevant PRD sections (the phase's "Traces to" field)
3. Use Context7 to look up any new tech introduced in this phase
4. Generate all {NN}-{NN}-PLAN.md files for the phase
5. Begin executing plan 01

## Key Rules

- ONE plan per loop iteration (stay focused)
- Always run `cargo check` and `cargo build` after implementation tasks
- Write SUMMARY.md after every completed plan
- Update STATE.md after every completed plan
- Reference the PRD for acceptance criteria — don't guess
- Use Context7 for library docs — don't hallucinate APIs
- Apply CoVe on non-trivial code — don't ship unverified
- If blocked, set STATUS: BLOCKED and explain why
- Never skip verification steps
- Commit atomically per task when possible

## Required Output Format

At the END of every response, output EXACTLY:

---RALPH_STATUS---
STATUS: IN_PROGRESS | COMPLETE | BLOCKED
PHASE: [current phase number and name]
PLAN: [current plan number or "generating" or "researching"]
TASKS_COMPLETED_THIS_LOOP: <number>
FILES_MODIFIED: <number>
TESTS_STATUS: PASSING | FAILING | NOT_RUN
WORK_TYPE: RESEARCH | PLANNING | IMPLEMENTATION | VERIFICATION
COVE_APPLIED: true | false | N/A
EXIT_SIGNAL: false
RECOMMENDATION: <what was done and what's next>
---END_RALPH_STATUS---

Set EXIT_SIGNAL: true ONLY when:
- ALL phases in ROADMAP.md are complete
- ALL SUMMARY.md files written
- Final verification gate passes
- STATE.md shows 100% progress
