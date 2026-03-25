# Roadmap — Milestone 1: MVP

## Progress
| Phase | Name | Status | Plans |
|-------|------|--------|-------|
| 1 | Project Scaffolding & Core Architecture | Not Started | TBD |
| 2 | SQLite Persistence Layer | Not Started | TBD |
| 3 | Agent Subprocess Management | Not Started | TBD |
| 4 | Chat UI & Message Rendering | Not Started | TBD |
| 5 | Thread & Project Management UI | Not Started | TBD |
| 6 | Multi-Session Tabs & Concurrency | Not Started | TBD |
| 7 | Settings, Polish & dev.sh | Not Started | TBD |

---

### Phase 1: Project Scaffolding & Core Architecture
**Goal**: Cargo workspace setup, iced app shell, basic window with dark theme, application state skeleton
**Depends on**: None
**Research**: Likely
**Research topics**: iced 0.13 API, workspace structure, theme customization
**Traces to**: PRD §4 (Stack, Architecture)

### Phase 2: SQLite Persistence Layer
**Goal**: Database schema, migrations, CRUD operations for projects, threads, messages, sessions, settings
**Depends on**: Phase 1
**Research**: Unlikely
**Research topics**: rusqlite API, migration patterns
**Traces to**: PRD §4 (Data Model), US-009

### Phase 3: Agent Subprocess Management
**Goal**: Spawn Claude Code and Codex as child processes, stdio JSON-RPC communication, streaming output parsing, session lifecycle (start/stop/interrupt), approval mode handling
**Depends on**: Phase 1
**Research**: Likely
**Research topics**: Claude Code CLI protocol, Codex CLI protocol (from t3code source), JSON-RPC over stdio
**Traces to**: PRD §3 Epic: Agent Sessions, Epic: Approval & Sandbox Modes (US-001 through US-006)

### Phase 4: Chat UI & Message Rendering
**Goal**: Chat view with scrollable message list, markdown rendering with syntax highlighting, composer input, streaming message updates
**Depends on**: Phase 1, Phase 3
**Research**: Likely
**Research topics**: iced text rendering, syntect integration, custom widgets
**Traces to**: PRD §3 US-002, PRD §5 Chat View + Composer

### Phase 5: Thread & Project Management UI
**Goal**: Sidebar with project groups, thread list, create/delete threads, thread selection, project directory assignment
**Depends on**: Phase 2, Phase 4
**Research**: Unlikely
**Traces to**: PRD §3 Epic: Thread & Project Management (US-007 through US-009), PRD §5 Sidebar

### Phase 6: Multi-Session Tabs & Concurrency
**Goal**: Tab bar for multiple active threads, concurrent agent sessions, independent session state per tab
**Depends on**: Phase 3, Phase 5
**Research**: Unlikely
**Traces to**: PRD §3 Epic: Multi-Session Concurrency (US-010), PRD §5 Tab Bar

### Phase 7: Settings, Polish & dev.sh
**Goal**: Settings panel, config file loading/saving, dev.sh script, error handling, startup optimization, final integration testing
**Depends on**: All previous phases
**Research**: Unlikely
**Traces to**: PRD §5 Settings, PRD §6 Non-Functional Requirements, PRD §7 MVP Scope
