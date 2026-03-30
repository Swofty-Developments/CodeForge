import { test, expect } from "@playwright/test";
import { injectMockIPC, seedData, gotoApp, selectThread } from "./helpers";

test.beforeEach(async ({ page }) => {
  await injectMockIPC(page);
});

test.describe("Tab bar", () => {
  test("tab appears when thread is opened", async ({ page }) => {
    await seedData(page, [
      { name: "Proj", path: "/tmp/p", threads: [{ title: "Tab Thread" }] },
    ]);
    await gotoApp(page);

    // No tabs initially
    await expect(page.locator(".tab")).toHaveCount(0);

    await selectThread(page, "Tab Thread");

    // A tab should now be visible
    await expect(page.locator(".tab")).toHaveCount(1);
    await expect(page.locator(".tab")).toContainText("Tab Thread");
  });

  test("tab can be closed", async ({ page }) => {
    await seedData(page, [
      {
        name: "Proj",
        path: "/tmp/p",
        threads: [{ title: "Closable" }, { title: "Stays" }],
      },
    ]);
    await gotoApp(page);

    // Open both threads
    await selectThread(page, "Closable");
    await selectThread(page, "Stays");
    await expect(page.locator(".tab")).toHaveCount(2);

    // Close the "Closable" tab using its close button
    const closableTab = page.locator(".tab", { hasText: "Closable" });
    await closableTab.locator(".tab-close").click();

    await expect(page.locator(".tab")).toHaveCount(1);
    await expect(page.locator(".tab")).toContainText("Stays");
  });

  test("active tab styling is applied", async ({ page }) => {
    await seedData(page, [
      {
        name: "Proj",
        path: "/tmp/p",
        threads: [{ title: "First" }, { title: "Second" }],
      },
    ]);
    await gotoApp(page);

    await selectThread(page, "First");
    await selectThread(page, "Second");

    // "Second" should be the active tab
    const activeTab = page.locator(".tab.active");
    await expect(activeTab).toHaveCount(1);
    await expect(activeTab).toContainText("Second");

    // Click "First" tab to switch
    await page.locator(".tab", { hasText: "First" }).click();

    const newActive = page.locator(".tab.active");
    await expect(newActive).toHaveCount(1);
    await expect(newActive).toContainText("First");
  });
});
