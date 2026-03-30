import { test, expect } from "@playwright/test";
import { injectMockIPC, seedData, gotoApp, createThread, selectThread } from "./helpers";

test.beforeEach(async ({ page }) => {
  await injectMockIPC(page);
});

test.describe("Sidebar", () => {
  test("renders with CodeForge title", async ({ page }) => {
    await gotoApp(page);

    const title = page.locator(".sidebar-title");
    await expect(title).toBeVisible();
    await expect(title).toHaveText("CodeForge");
  });

  test("new thread button creates a thread", async ({ page }) => {
    await gotoApp(page);

    // Initially no thread items
    await expect(page.locator(".thread-item")).toHaveCount(0);

    await createThread(page);

    // A thread item should now appear
    await expect(page.locator(".thread-item")).toHaveCount(1);
  });

  test("thread appears in sidebar after creation", async ({ page }) => {
    await gotoApp(page);

    await createThread(page);

    const item = page.locator(".thread-item").first();
    await expect(item).toBeVisible();
    // The default title is "Thread N"
    await expect(item).toContainText("Thread");
  });

  test("thread can be selected and becomes active", async ({ page }) => {
    await seedData(page, [
      {
        name: "TestProject",
        path: "/tmp/test",
        threads: [{ title: "Alpha" }, { title: "Beta" }],
      },
    ]);
    await gotoApp(page);

    // Select "Beta"
    await selectThread(page, "Beta");

    const active = page.locator(".thread-item.active");
    await expect(active).toHaveCount(1);
    await expect(active).toContainText("Beta");
  });

  test("project groups can collapse and expand", async ({ page }) => {
    await seedData(page, [
      {
        name: "MyProject",
        path: "/tmp/proj",
        threads: [{ title: "Thread A" }],
      },
    ]);
    await gotoApp(page);

    // Thread should be visible initially
    await expect(page.locator(".thread-item").first()).toBeVisible();

    // Click the project toggle to collapse
    await page.click(".project-toggle");

    // Thread should now be hidden
    await expect(page.locator(".thread-item")).toHaveCount(0);

    // Click again to expand
    await page.click(".project-toggle");
    await expect(page.locator(".thread-item")).toHaveCount(1);
  });
});
