# Architecture

## Overview
CodeForge follows an Elm-like architecture (Model-Update-View) native to iced, with a session manager layer that handles async subprocess communication.

## Layers

### 1. GUI Layer (iced)
- **App state** (Model): threads, messages, sessions, UI state
- **Message enum**: all UI events and async results
- **View functions**: sidebar, chat, composer, tabs, settings
- **Subscriptions**: agent output streams, timers

### 2. Session Manager
- Manages lifecycle of agent subprocesses
- One session = one child process (claude or codex)
- Handles stdio pipe reading/writing
- Translates between JSON-RPC and app Messages
- Runs on tokio, bridges to iced via Subscription

### 3. Persistence Layer
- SQLite via rusqlite
- CRUD for projects, threads, messages, sessions
- Migrations on startup
- Sync operations (rusqlite is not async, use spawn_blocking)

### 4. Config Layer
- TOML config at ~/.codeforge/config.toml
- Runtime settings in SQLite settings table
- XDG-compliant directory structure

## Data Flow

```
User Input → iced Message → update() →
  ├── UI state change → view() re-render
  ├── DB write (spawn_blocking) → completion Message
  └── Session command → subprocess stdin write →
      subprocess stdout read → iced Subscription →
      Message::AgentOutput → update() → view()
```

## Key Patterns
- **Elm architecture**: immutable state, pure view functions, side effects via Command/Subscription
- **Actor-like sessions**: each agent session is an independent async task
- **Channel bridges**: tokio mpsc channels bridge async subprocess IO to iced subscriptions
