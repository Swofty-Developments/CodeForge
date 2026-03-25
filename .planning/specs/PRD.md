# PRD: CodeForge

## 1. Overview

CodeForge is a native Linux desktop application written in Rust that provides a unified GUI for interacting with AI coding agents (Claude Code and Codex). Inspired by [t3code](https://github.com/pingdotgg/t3code), it offers session management, threaded conversations, project organization, and multi-agent concurrency — all in a fast, native iced-based interface with a dark IDE-like aesthetic. The app launches via `./dev.sh` and communicates with agents through subprocess stdio/JSON-RPC.

## 2. Target Users

| User Type | Description | Primary Need |
|-----------|-------------|--------------|
| AI-assisted developer | Uses Claude Code or Codex daily for coding tasks | Fast, organized interface to manage multiple agent sessions across projects |
| Multi-project developer | Works across several repos simultaneously | Thread/project organization with persistent history |
| Power user | Wants full control over agent behavior | Toggle between supervised and auto-approve modes per session |

## 3. User Stories

### Epic: Agent Sessions
- **US-001**: As a developer, I want to start a new agent session (Claude Code or Codex) so that I can begin an AI-assisted coding task
  - Acceptance Criteria:
    - [ ] Can select provider (Claude Code or Codex) when starting a session
    - [ ] Agent process spawns as a subprocess with stdio communication
    - [ ] Session appears in the active sessions list
- **US-002**: As a developer, I want to send messages to the agent and see streaming responses so that I can have a real-time conversation
  - Acceptance Criteria:
    - [ ] Text input composer at the bottom of the chat view
    - [ ] Responses stream in token-by-token as they arrive
    - [ ] Messages are rendered with markdown formatting and syntax highlighting
- **US-003**: As a developer, I want to interrupt a running agent so that I can stop unwanted actions
  - Acceptance Criteria:
    - [ ] Interrupt button visible during active generation
    - [ ] Agent process receives appropriate signal to stop
    - [ ] UI reflects interrupted state
- **US-004**: As a developer, I want to resume or stop sessions so that I can manage agent lifecycle
  - Acceptance Criteria:
    - [ ] Can resume a paused session
    - [ ] Can permanently stop/kill a session
    - [ ] Session state persists across app restarts (via SQLite)

### Epic: Approval & Sandbox Modes
- **US-005**: As a developer, I want to toggle between supervised and auto-approve modes so that I can control agent autonomy
  - Acceptance Criteria:
    - [ ] Per-session toggle for approval mode
    - [ ] In supervised mode, agent actions requiring approval show a prompt
    - [ ] In auto-approve mode, agent runs without interruption
    - [ ] Default mode is configurable in settings
- **US-006**: As a developer, I want to approve or deny individual agent actions in supervised mode
  - Acceptance Criteria:
    - [ ] Approval prompt shows the action description and affected files
    - [ ] Can approve or deny with keyboard shortcuts
    - [ ] Denied actions are communicated back to the agent

### Epic: Thread & Project Management
- **US-007**: As a developer, I want to organize conversations into threads so that I can keep track of different tasks
  - Acceptance Criteria:
    - [ ] Each session belongs to a thread
    - [ ] Threads have titles (auto-generated or user-set)
    - [ ] Thread list in sidebar with timestamps
- **US-008**: As a developer, I want to group threads by project directory so that I can navigate between projects
  - Acceptance Criteria:
    - [ ] Sidebar shows project directories as groups
    - [ ] Threads are nested under their project
    - [ ] Can filter/search threads
- **US-009**: As a developer, I want threads and messages to persist across app restarts
  - Acceptance Criteria:
    - [ ] All data stored in SQLite at `~/.codeforge/codeforge.db`
    - [ ] App loads previous threads on startup
    - [ ] Can delete threads

### Epic: Multi-Session Concurrency
- **US-010**: As a power user, I want to run multiple agent sessions simultaneously so that I can work on different tasks in parallel
  - Acceptance Criteria:
    - [ ] Multiple tabs/panes for concurrent sessions
    - [ ] Each session has its own agent subprocess
    - [ ] Sessions are independent (different providers, projects, modes)
    - [ ] Resource usage is reasonable with multiple agents running

## 4. Technical Requirements

### Stack
| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Language | Rust (stable) | Performance, safety, native Linux support |
| GUI | iced 0.13+ | Elm-inspired, async-native, great Linux/Wayland support |
| Database | SQLite via rusqlite | Lightweight, embedded, t3code uses SQLite too |
| Async Runtime | tokio | Standard Rust async, needed for subprocess management |
| Subprocess IO | tokio::process | Async child process spawn + stdio pipes |
| Serialization | serde + serde_json | JSON-RPC message parsing |
| Markdown Rendering | Custom iced widgets | Render markdown in chat messages |
| Syntax Highlighting | syntect | Code block highlighting in messages |
| Logging | tracing + tracing-subscriber | Structured logging |
| Config | toml + directories | App config in `~/.codeforge/config.toml` |

### Architecture

```
┌─────────────────────────────────────────────────┐
│                   iced GUI                       │
│  ┌──────────┐  ┌─────────────┐  ┌────────────┐ │
│  │ Sidebar  │  │  Chat View  │  │  Settings   │ │
│  │ Projects │  │  Messages   │  │  Panel      │ │
│  │ Threads  │  │  Composer   │  │             │ │
│  └──────────┘  └─────────────┘  └────────────┘ │
└─────────────┬───────────────────────────────────┘
              │ Messages (iced::Subscription)
┌─────────────▼───────────────────────────────────┐
│              Session Manager                     │
│  ┌─────────────┐  ┌─────────────┐               │
│  │ Session 1   │  │ Session 2   │  ...          │
│  │ (Claude)    │  │ (Codex)     │               │
│  └──────┬──────┘  └──────┬──────┘               │
└─────────┼────────────────┼──────────────────────┘
          │ stdio/JSON-RPC │
┌─────────▼──────┐ ┌──────▼──────────┐
│  claude code   │ │  codex CLI      │
│  subprocess    │ │  subprocess     │
└────────────────┘ └─────────────────┘

┌─────────────────────────────────────────────────┐
│              SQLite Persistence                  │
│  threads, messages, sessions, settings           │
└─────────────────────────────────────────────────┘
```

### Data Model
| Entity | Key Fields | Relationships |
|--------|-----------|---------------|
| Project | id, path, name, created_at | has many Threads |
| Thread | id, project_id, title, created_at, updated_at | belongs to Project, has many Messages |
| Message | id, thread_id, role (user/assistant/system), content, created_at | belongs to Thread |
| Session | id, thread_id, provider (claude/codex), status, approval_mode, pid | belongs to Thread |
| Settings | key, value | global app settings |

## 5. Screens & Navigation

### Screen Map
```
App
├── Sidebar (always visible, left)
│   ├── Project list (collapsible groups)
│   │   └── Thread list (per project)
│   ├── New thread button
│   └── Settings button
├── Main Panel (center)
│   ├── Tab bar (one tab per active thread)
│   ├── Chat View (messages + streaming)
│   └── Composer (input + send + provider selector)
└── Settings Panel (overlay/modal)
    ├── Default provider
    ├── Default approval mode
    ├── Theme settings
    └── Agent paths (claude/codex binary locations)
```

### Screen Descriptions
| Screen | Purpose | Key Components |
|--------|---------|----------------|
| Sidebar | Navigate projects and threads | Collapsible project groups, thread list with timestamps, new thread button |
| Chat View | Display conversation with agent | Scrollable message list, markdown rendering, code blocks with syntax highlighting |
| Composer | Send messages to agent | Text input, send button, provider selector dropdown, approval mode toggle |
| Tab Bar | Switch between active threads | Closeable tabs, active indicator, provider icon per tab |
| Settings | Configure app behavior | Form inputs for paths, dropdowns for defaults, theme toggle |

## 6. Non-Functional Requirements

- **Startup time**: < 500ms to window visible
- **Message latency**: < 50ms from agent output to screen render
- **Memory**: < 100MB base, < 50MB per active session
- **Binary size**: < 30MB release build
- **Platform**: Linux x86_64 (primary), aarch64 (secondary)
- **Accessibility**: Keyboard navigable, high contrast in dark theme
- **Config directory**: `~/.codeforge/`

## 7. MVP Scope

### In Scope (MVP)
- Start/stop/interrupt agent sessions (Claude Code + Codex)
- Streaming chat with markdown rendering
- Thread management with SQLite persistence
- Project grouping by directory
- Multi-session tabs
- Supervised + auto-approve modes
- Dark IDE-like theme
- `./dev.sh` launcher script
- Basic settings (provider paths, default mode)

### Out of Scope (Post-MVP)
- Code diff visualization
- Embedded terminal / PTY
- Git integration (checkpoints, branch management)
- File attachment support
- Drag-and-drop thread reordering
- Remote access / network mode
- Auto-update mechanism
- Keybinding customization
- Light theme

## 8. Success Metrics

| Metric | Target | How Measured |
|--------|--------|--------------|
| Can start Claude Code session | Works | Manual test |
| Can start Codex session | Works | Manual test |
| Streaming responses render | < 50ms latency | Visual inspection |
| Threads persist across restart | Works | Kill + relaunch |
| Multiple sessions run concurrently | 3+ simultaneous | Manual test |
| App builds and runs via ./dev.sh | Works | Fresh clone test |

## 9. Open Questions

- Exact JSON-RPC protocol for Codex stdio communication (need to inspect t3code's CodexAdapter)
- Claude Code subprocess communication protocol (need to inspect t3code's ClaudeAdapter)
- Whether to use iced's built-in text rendering or a custom rich text widget for markdown
