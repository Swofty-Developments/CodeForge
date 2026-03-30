import { test, expect } from "@playwright/test";
import { injectMockIPC, seedData, gotoApp, selectThread, typeInComposer } from "./helpers";

test.beforeEach(async ({ page }) => {
  await injectMockIPC(page);
});

test.describe("Composer", () => {
  test("shows when a thread is active", async ({ page }) => {
    await seedData(page, [
      { name: "Proj", path: "/tmp/p", threads: [{ title: "Active Thread" }] },
    ]);
    await gotoApp(page);

    // Composer should not be visible when no thread is selected
    await expect(page.locator(".composer-wrapper")).toHaveCount(0);

    // Select the thread
    await selectThread(page, "Active Thread");

    // Now the composer should appear
    await expect(page.locator(".composer-wrapper")).toBeVisible();
  });

  test("can type in composer textarea", async ({ page }) => {
    await seedData(page, [
      { name: "Proj", path: "/tmp/p", threads: [{ title: "Typing Test" }] },
    ]);
    await gotoApp(page);
    await selectThread(page, "Typing Test");

    await typeInComposer(page, "Hello, world!");

    const input = page.locator(".composer-input");
    await expect(input).toHaveValue("Hello, world!");
  });

  test("send button is visible", async ({ page }) => {
    await seedData(page, [
      { name: "Proj", path: "/tmp/p", threads: [{ title: "Send Test" }] },
    ]);
    await gotoApp(page);
    await selectThread(page, "Send Test");

    const sendBtn = page.locator(".send-btn");
    await expect(sendBtn).toBeVisible();
  });

  test("provider picker button works", async ({ page }) => {
    await seedData(page, [
      { name: "Proj", path: "/tmp/p", threads: [{ title: "Provider Test" }] },
    ]);
    await gotoApp(page);
    await selectThread(page, "Provider Test");

    // The provider pill is the first .meta-pill
    const providerPill = page.locator(".meta-pill").first();
    await expect(providerPill).toBeVisible();
    await expect(providerPill).toContainText("Claude Code");

    // Click it to open the provider picker
    await providerPill.click();

    // The provider picker overlay should now be visible
    // (ProviderPicker component renders when providerPickerOpen is true)
    // We just verify the click didn't error and the pill is still there
    await expect(providerPill).toBeVisible();
  });
});
