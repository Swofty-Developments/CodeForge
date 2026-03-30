import { test, expect } from "@playwright/test";
import { injectMockIPC, seedData, gotoApp, selectThread, sendMessageAndWait } from "./helpers";

test.beforeEach(async ({ page }) => {
  await injectMockIPC(page);
});

test.describe("Chat area", () => {
  test('shows "New conversation" when thread is open with no messages', async ({ page }) => {
    await seedData(page, [
      { name: "Proj", path: "/tmp/p", threads: [{ title: "Empty Thread" }] },
    ]);
    await gotoApp(page);
    await selectThread(page, "Empty Thread");

    const empty = page.locator(".chat-empty");
    await expect(empty).toBeVisible();
    await expect(empty).toContainText("New conversation");
  });

  test("renders messages for a thread", async ({ page }) => {
    await seedData(page, [
      {
        name: "Proj",
        path: "/tmp/p",
        threads: [
          {
            title: "Chat Thread",
            messages: [
              { role: "user", content: "What is Rust?" },
              { role: "assistant", content: "Rust is a systems programming language." },
            ],
          },
        ],
      },
    ]);
    await gotoApp(page);
    await selectThread(page, "Chat Thread");

    // Wait for messages to render
    await expect(page.locator(".message")).toHaveCount(2);
  });

  test("messages have correct role classes", async ({ page }) => {
    await seedData(page, [
      {
        name: "Proj",
        path: "/tmp/p",
        threads: [
          {
            title: "Role Thread",
            messages: [
              { role: "user", content: "Hello" },
              { role: "assistant", content: "Hi there" },
              { role: "system", content: "System notice" },
            ],
          },
        ],
      },
    ]);
    await gotoApp(page);
    await selectThread(page, "Role Thread");

    await expect(page.locator(".message")).toHaveCount(3);
    await expect(page.locator(".message-user")).toHaveCount(1);
    await expect(page.locator(".message-assistant")).toHaveCount(1);
    await expect(page.locator(".message-system")).toHaveCount(1);
  });

  test("sending a message shows user bubble and streams assistant response", async ({ page }) => {
    await seedData(page, [
      { name: "Proj", path: "/tmp/p", threads: [{ title: "Send Test" }] },
    ]);
    await gotoApp(page);
    await selectThread(page, "Send Test");

    // Send a message — the mock IPC will emit streaming events
    await sendMessageAndWait(page, "Hello Claude!");

    // User message should appear
    const userMsg = page.locator(".message-user");
    await expect(userMsg).toHaveCount(1);
    await expect(userMsg).toContainText("Hello Claude!");

    // Assistant response should appear (streamed via mock agent events)
    const assistantMsg = page.locator(".message-assistant");
    await expect(assistantMsg).toHaveCount(1);
    // The mock responds with: 'This is a mock response to: "Hello Claude!"'
    await expect(assistantMsg).toContainText("mock response");

    // Streaming should be done — no cursor visible
    await expect(page.locator(".cursor")).toHaveCount(0);
  });
});
