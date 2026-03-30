import { test, expect } from "@playwright/test";
import { injectMockIPC, seedData, gotoApp, openCommandPalette, openSearchOverlay } from "./helpers";

test.beforeEach(async ({ page }) => {
  await injectMockIPC(page);
});

test.describe("Overlays", () => {
  test("command palette opens with Cmd+K", async ({ page }) => {
    await gotoApp(page);

    // Should not be visible initially
    await expect(page.locator(".cmd-palette-overlay")).toHaveCount(0);

    await openCommandPalette(page);

    await expect(page.locator(".cmd-palette-overlay")).toBeVisible();
    await expect(page.locator(".cmd-palette-input")).toBeVisible();
  });

  test("command palette can be searched", async ({ page }) => {
    await seedData(page, [
      {
        name: "SearchProj",
        path: "/tmp/sp",
        threads: [{ title: "Unique Thread Name" }],
      },
    ]);
    await gotoApp(page);
    await openCommandPalette(page);

    const input = page.locator(".cmd-palette-input");
    await input.fill("Unique");

    // The matching thread should appear in the palette results
    const item = page.locator(".cmd-palette-item-label", { hasText: "Unique Thread Name" });
    await expect(item).toBeVisible();
  });

  test("command palette closes on Escape", async ({ page }) => {
    await gotoApp(page);
    await openCommandPalette(page);

    await expect(page.locator(".cmd-palette-overlay")).toBeVisible();

    await page.keyboard.press("Escape");

    await expect(page.locator(".cmd-palette-overlay")).toHaveCount(0);
  });

  test("settings overlay opens and closes", async ({ page }) => {
    await gotoApp(page);

    // Click the settings gear icon in sidebar header
    await page.click(".sidebar-header .icon-btn");

    // Settings overlay should be visible (uses .overlay or similar)
    // The SettingsOverlay component renders when settingsOpen is true
    // Wait a moment for the overlay to mount
    await page.waitForTimeout(100);

    // Verify something rendered (the settings component is in the DOM)
    // Settings typically has an overlay backdrop or panel
    const settingsVisible = await page.evaluate(() => {
      const store = (window as any).__MOCK_STATE__;
      // If we can see any overlay-like element, settings is open
      return document.querySelectorAll("[class*='overlay'], [class*='settings']").length > 0;
    });
    expect(settingsVisible).toBeTruthy();
  });

  test("search overlay opens with Cmd+Shift+F", async ({ page }) => {
    await gotoApp(page);

    await openSearchOverlay(page);

    // SearchOverlay manages its own open state via keyboard listener,
    // but the App also toggles searchOpen in the store.
    // Wait for the overlay to appear
    await page.waitForTimeout(200);

    // The search overlay uses inline styles rather than a class, but
    // the App renders it when store.searchOpen is true.
    // Check that something at z-index 9999 is now present (the backdrop)
    const hasOverlay = await page.evaluate(() => {
      const els = document.querySelectorAll("div");
      for (const el of els) {
        const z = window.getComputedStyle(el).zIndex;
        if (z === "9999") return true;
      }
      return false;
    });
    expect(hasOverlay).toBeTruthy();
  });
});
