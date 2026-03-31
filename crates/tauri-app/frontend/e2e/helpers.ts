import { type Page } from "@playwright/test";

// ────────────────────────────────────────────────────────
// In-memory data that the mock IPC layer manages
// ────────────────────────────────────────────────────────

interface MockProject {
  id: string;
  name: string;
  path: string;
  color: string | null;
}

interface MockThread {
  id: string;
  project_id: string;
  title: string;
  color: string | null;
}

interface MockMessage {
  id: string;
  thread_id: string;
  role: "user" | "assistant" | "system";
  content: string;
}

/**
 * Inject a mock Tauri IPC layer into the page so the SolidJS frontend can run
 * without the Rust backend.  The mock handles every `invoke` command used by
 * `src/ipc.ts` and keeps state in `window.__MOCK_STATE__` for assertions.
 */
export async function injectMockIPC(page: Page) {
  await page.addInitScript(() => {
    // ---------- state ----------
    const state: {
      projects: MockProject[];
      threads: MockThread[];
      messages: MockMessage[];
      settings: Record<string, string>;
      nextId: number;
    } = {
      projects: [],
      threads: [],
      messages: [],
      settings: {},
      nextId: 1,
    };

    function uid() {
      return `mock-${state.nextId++}`;
    }

    // Expose state so Playwright assertions can inspect it
    (window as any).__MOCK_STATE__ = state;

    // ---------- mock invoke ----------
    async function mockInvoke(cmd: string, args: any = {}): Promise<any> {
      switch (cmd) {
        case "get_all_projects":
          return state.projects;

        case "get_threads_by_project":
          return state.threads.filter((t) => t.project_id === args.projectId);

        case "get_messages_by_thread":
          return state.messages.filter((m) => m.thread_id === args.threadId);

        case "create_project": {
          const p: MockProject = { id: uid(), name: args.name, path: args.path, color: null };
          state.projects.push(p);
          return p;
        }

        case "rename_project":
          for (const p of state.projects) if (p.id === args.id) p.name = args.name;
          return null;

        case "delete_project":
          state.projects = state.projects.filter((p) => p.id !== args.id);
          if (args.deleteThreads) state.threads = state.threads.filter((t) => t.project_id !== args.id);
          return null;

        case "create_thread": {
          const t: MockThread = {
            id: uid(),
            project_id: args.projectId,
            title: args.title,
            color: null,
          };
          state.threads.push(t);
          return t;
        }

        case "rename_thread":
          for (const t of state.threads) if (t.id === args.id) t.title = args.title;
          return null;

        case "set_thread_color":
          for (const t of state.threads) if (t.id === args.id) t.color = args.color;
          return null;

        case "delete_thread":
          state.threads = state.threads.filter((t) => t.id !== args.id);
          return null;

        case "move_thread_to_project":
          for (const t of state.threads) if (t.id === args.threadId) t.project_id = args.targetProjectId;
          return null;

        case "persist_user_message": {
          const id = uid();
          state.messages.push({
            id,
            thread_id: args.threadId,
            role: "user",
            content: args.content,
          });
          return id;
        }

        case "send_message": {
          // Simulate a streamed assistant response via agent-event emissions.
          // The real backend emits Tauri events; here we fire the registered
          // listener callback directly after a small delay to mimic latency.
          const threadId = args.threadId as string;
          const sessionId = `mock-session-${state.nextId++}`;
          const responseText = `This is a mock response to: "${args.text}"`;

          setTimeout(() => {
            emitAgentEvent({
              session_id: sessionId,
              thread_id: threadId,
              event_type: "turn_started",
              turn_id: "turn-1",
            });

            // Stream the response word by word
            const words = responseText.split(" ");
            words.forEach((word, i) => {
              setTimeout(() => {
                emitAgentEvent({
                  session_id: sessionId,
                  thread_id: threadId,
                  event_type: "content_delta",
                  text: (i > 0 ? " " : "") + word,
                });

                // After last word, emit turn_completed
                if (i === words.length - 1) {
                  setTimeout(() => {
                    emitAgentEvent({
                      session_id: sessionId,
                      thread_id: threadId,
                      event_type: "turn_completed",
                      turn_id: "turn-1",
                    });
                  }, 50);
                }
              }, i * 30); // 30ms per word
            });
          }, 100);

          return null;
        }

        case "stop_session":
          return null;

        case "respond_to_approval":
          return null;

        case "get_setting":
          return state.settings[args.key] ?? null;

        case "set_setting":
          state.settings[args.key] = args.value;
          return null;

        case "get_provider_info":
          return [
            {
              id: "claude_code",
              name: "Claude Code",
              installed: true,
              path: "/usr/bin/claude",
              version: "1.0.0",
              install_instructions: "",
              description: "Anthropic Claude Code",
              website: "https://anthropic.com",
            },
          ];

        case "create_worktree":
          return { thread_id: args.threadId, branch: "test-branch", path: "/tmp/worktree", active: true };

        case "get_worktree":
          return null;

        case "merge_worktree":
          return "Merged successfully";

        case "search_messages":
          return state.messages
            .filter((m) => m.content.toLowerCase().includes((args.query as string).toLowerCase()))
            .map((m) => ({
              thread_id: m.thread_id,
              thread_title: state.threads.find((t) => t.id === m.thread_id)?.title ?? "",
              project_name: "",
              message_id: m.id,
              role: m.role,
              content_snippet: m.content.slice(0, 80),
              match_index: 0,
            }));

        case "get_usage_summary":
          return {
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_read_tokens: 0,
            total_cache_write_tokens: 0,
            total_cost_usd: 0,
            thread_costs: [],
            model_costs: [],
          };

        case "get_thread_usage":
          return {
            thread_id: args.threadId,
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0,
          };

        // Onboarding
        case "check_setup_status":
          return {
            complete: true,
            binaries: [{ name: "claude", installed: true, version: "1.0.0", path: "/usr/bin/claude" }],
            has_any_binary: true,
            gh_installed: true,
            gh_authenticated: true,
            gh_username: "test-user",
          };
        case "complete_setup":
          return null;

        // GitHub
        case "gh_auth_status":
          return { logged_in: true, username: "test-user", scopes: ["repo"] };
        case "list_prs":
        case "list_issues":
          return [];
        case "is_github_repo":
          return false;
        case "get_issue_context":
        case "get_pr_diff":
        case "get_repo_info":
          return null;
        case "gh_login":
          return "ok";

        // MCP
        case "mcp_list_servers":
          return [];
        case "mcp_add_server":
        case "mcp_remove_server":
          return "ok";
        case "list_slash_commands":
          return [{ name: "/help", description: "Show help", source: "built-in" }];

        // Diff
        case "get_changed_files":
        case "get_session_diff":
          return [];
        case "get_file_diff":
        case "get_file_content":
          return "";

        // Naming
        case "auto_name_thread":
          return null;

        default:
          console.warn(`[mock-ipc] unhandled command: ${cmd}`, args);
          return null;
      }
    }

    // ---------- Agent event emission ----------
    // Stores registered event listener callbacks so mock send_message can fire them.
    const eventListeners: Record<string, ((...args: any[]) => void)[]> = {};

    function emitAgentEvent(payload: any) {
      const cbs = eventListeners["agent-event"] || [];
      for (const cb of cbs) {
        try {
          cb({ event: "agent-event", id: 0, payload });
        } catch (e) {
          console.error("[mock-ipc] emitAgentEvent error:", e);
        }
      }
    }

    // Expose emitAgentEvent for direct use from tests
    (window as any).__MOCK_EMIT__ = emitAgentEvent;

    // ---------- Tauri IPC shim ----------
    // @tauri-apps/api/core reads from window.__TAURI_INTERNALS__
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args: any) => {
        // The `listen` helper first invokes `plugin:event|listen`
        if (cmd === "plugin:event|listen") {
          // args contains: { event, handler (callback id), target }
          // The handler is a callback id registered via transformCallback
          const event = args?.event as string;
          const handlerId = args?.handler;
          if (event && handlerId != null) {
            const callbackFn = (window as any)[`_${handlerId}`];
            if (callbackFn) {
              if (!eventListeners[event]) eventListeners[event] = [];
              eventListeners[event].push(callbackFn);
            }
          }
          return Promise.resolve(Number(uid().replace("mock-", "")));
        }
        return mockInvoke(cmd, args);
      },
      transformCallback(callback: (...args: any[]) => void) {
        const id = uid();
        (window as any)[`_${id}`] = callback;
        return id;
      },
      metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    };

    // Types for TS (unused at runtime)
    interface MockProject { id: string; name: string; path: string; color: string | null }
    interface MockThread { id: string; project_id: string; title: string; color: string | null }
    interface MockMessage { id: string; thread_id: string; role: string; content: string }
  });
}

// ────────────────────────────────────────────────────────
// Pre-seed helpers: inject data *before* the page loads
// ────────────────────────────────────────────────────────

export interface SeedProject {
  name: string;
  path?: string;
  threads?: { title: string; messages?: { role: "user" | "assistant" | "system"; content: string }[] }[];
}

/**
 * Seed the mock IPC with projects/threads/messages, then navigate.
 * Must be called AFTER `injectMockIPC` and BEFORE `page.goto`.
 */
export async function seedData(page: Page, projects: SeedProject[]) {
  await page.addInitScript((data: SeedProject[]) => {
    // This runs before app code; __MOCK_STATE__ is already set by injectMockIPC
    function waitForState(cb: () => void) {
      const check = () => {
        if ((window as any).__MOCK_STATE__) {
          cb();
        } else {
          setTimeout(check, 5);
        }
      };
      check();
    }

    waitForState(() => {
      const s = (window as any).__MOCK_STATE__;
      let id = 100;
      for (const p of data) {
        const pid = `seed-p-${id++}`;
        s.projects.push({ id: pid, name: p.name, path: p.path ?? ".", color: null });
        if (p.threads) {
          for (const t of p.threads) {
            const tid = `seed-t-${id++}`;
            s.threads.push({ id: tid, project_id: pid, title: t.title, color: null });
            if (t.messages) {
              for (const m of t.messages) {
                s.messages.push({ id: `seed-m-${id++}`, thread_id: tid, role: m.role, content: m.content });
              }
            }
          }
        }
      }
    });
  }, projects);
}

// ────────────────────────────────────────────────────────
// High-level action helpers
// ────────────────────────────────────────────────────────

/** Click the "+ New Thread" button in the sidebar footer. */
export async function createThread(page: Page) {
  await page.click(".new-thread-btn");
}

/** Click a thread item in the sidebar to select it. Uses force:true because dnd-kit sets aria-disabled on non-draggable items. */
export async function selectThread(page: Page, title: string) {
  await page.locator(`.thread-item:has-text("${title}")`).click({ force: true });
}

/** Type a message into the composer and optionally send it. */
export async function typeInComposer(page: Page, text: string, send = false) {
  await page.fill(".composer-input", text);
  if (send) {
    await page.click(".send-btn");
  }
}

/** Type a message, send it, and wait for the mock streaming response to complete. */
export async function sendMessageAndWait(page: Page, text: string) {
  await page.fill(".composer-input", text);
  await page.click(".send-btn");
  // Wait for the turn_completed to fire — the assistant message should get a done- prefix
  // and the streaming cursor should disappear. Give it enough time for the mock delays.
  await page.waitForTimeout(800);
}

/** Emit an agent event directly from the test (bypass send_message mock). */
export async function emitAgentEvent(page: Page, payload: Record<string, any>) {
  await page.evaluate((p) => {
    (window as any).__MOCK_EMIT__(p);
  }, payload);
}

/** Open the command palette via keyboard shortcut. */
export async function openCommandPalette(page: Page) {
  await page.keyboard.press("Meta+k");
}

/** Open the search overlay via keyboard shortcut. */
export async function openSearchOverlay(page: Page) {
  await page.keyboard.press("Meta+Shift+f");
}

/** Navigate to the app, waiting for it to settle. Dismisses the welcome screen if present. */
export async function gotoApp(page: Page) {
  await page.goto("/");
  // Wait for the SolidJS app to mount
  await page.waitForSelector("#app");
  // Small grace period for reactive hydration
  await page.waitForTimeout(300);
  // Dismiss the welcome screen if it exists
  const welcome = page.locator(".ws");
  if (await welcome.count() > 0) {
    await welcome.click();
    await page.waitForTimeout(500); // wait for exit animation
  }
}
