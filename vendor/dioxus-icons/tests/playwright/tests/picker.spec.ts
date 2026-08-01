import { test, expect, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";
import path from "node:path";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const DOC_INDEX = path.resolve(
  HERE,
  "../../../target/doc/dioxus_icons/index.html",
);
const DOC_URL = "file://" + DOC_INDEX;
const TRASH_DOC = path.resolve(
  HERE,
  "../../../target/doc/dioxus_icons/lucide/fn.Trash.html",
);
const TRASH_URL = "file://" + TRASH_DOC;
const MAX_PICKER_READY_MS = Number(process.env.DXI_PICKER_READY_MS ?? 10_000);
const MAX_PICKER_SEARCH_MS = Number(process.env.DXI_PICKER_SEARCH_MS ?? 750);

async function waitForInteractivePicker(page: Page) {
  await page.locator("#dxi-picker:not([hidden])").waitFor();
  await page
    .locator("#dxi-picker-grid .dxi-picker-cell")
    .first()
    .waitFor();
}

async function visiblePickerNames(page: Page, limit: number): Promise<string[]> {
  return await page
    .locator("#dxi-picker-grid .dxi-picker-cell-name")
    .evaluateAll((names, max) =>
      names
        .slice(0, max)
        .map((el) => (el.textContent ?? "").trim()),
      limit,
    );
}

test.describe("docs homepage picker", () => {
  test("interactive picker becomes visible and replaces the static fallback", async ({
    page,
  }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    await expect(page.locator("#dxi-picker")).toBeVisible();

    const fallbackDisplay = await page
      .locator("#dioxus-icons-static-picker")
      .evaluate((el) => getComputedStyle(el).display);
    expect(fallbackDisplay).toBe("none");
  });

  test("interactive grid lays out cells in multiple columns", async ({
    page,
  }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const grid = page.locator("#dxi-picker-grid");
    const trackCount = await grid.evaluate(
      (el) =>
        getComputedStyle(el)
          .gridTemplateColumns.split(" ")
          .filter((token) => token.length > 0).length,
    );
    expect(trackCount).toBeGreaterThan(2);

    const cellCount = await grid.locator(".dxi-picker-cell").count();
    expect(cellCount).toBeGreaterThanOrEqual(trackCount);
  });

  test("first row cells share a single offsetTop (JS columns match CSS)", async ({
    page,
  }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const grid = page.locator("#dxi-picker-grid");
    const trackCount = await grid.evaluate(
      (el) =>
        getComputedStyle(el)
          .gridTemplateColumns.split(" ")
          .filter((token) => token.length > 0).length,
    );

    const tops = await grid
      .locator(".dxi-picker-cell")
      .evaluateAll((cells) =>
        cells.slice(0, 32).map((el) => (el as HTMLElement).offsetTop),
      );

    const firstRowTop = tops[0];
    const cellsInFirstRow = tops.filter((t) => t === firstRowTop).length;
    expect(cellsInFirstRow).toBe(trackCount);

    const distinctTops = new Set(tops).size;
    expect(distinctTops).toBeGreaterThan(1);
  });

  test("typing in the search narrows the result count", async ({ page }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const count = page.locator("#dxi-picker-count strong");
    const total = Number(await count.innerText());
    expect(total).toBeGreaterThan(100);

    await page.locator("#dxi-picker-input").fill("trash");
    await expect
      .poll(async () => Number(await count.innerText()))
      .toBeLessThan(total);

    const firstHref = await page
      .locator("#dxi-picker-grid .dxi-picker-cell")
      .first()
      .getAttribute("href");
    expect(firstHref).toMatch(/lucide\/fn\.[A-Za-z0-9_]*[Tt]rash[A-Za-z0-9_]*\.html/);
  });

  test("search ranking prefers icon-name matches before tag-only matches", async ({
    page,
  }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    await page.locator("#dxi-picker-input").fill("trash");
    await expect(page.locator("#dxi-picker-count strong")).toHaveText("4");

    const names = await visiblePickerNames(page, 4);
    expect(names).toEqual(["Trash", "Trash2", "Eraser", "Shredder"]);
  });

  test("clear button visibility and click restore the full result set", async ({
    page,
  }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    const clear = page.locator("#dxi-picker-clear");
    const count = page.locator("#dxi-picker-count strong");
    const total = Number(await count.innerText());
    await expect(clear).toBeHidden();

    await input.fill("trash");
    await expect
      .poll(async () => Number(await count.innerText()))
      .toBeLessThan(total);
    await expect(clear).toBeVisible();

    await clear.click();
    await expect(input).toBeFocused();
    await expect(input).toHaveValue("");
    await expect(clear).toBeHidden();
    await expect.poll(async () => Number(await count.innerText())).toBe(total);
  });

  test("Enter on the clear button clears the search", async ({ page }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    const count = page.locator("#dxi-picker-count strong");
    const total = Number(await count.innerText());

    await input.fill("trash");
    await expect
      .poll(async () => Number(await count.innerText()))
      .toBeLessThan(total);

    await page.locator("#dxi-picker-clear").focus();
    await page.keyboard.press("Enter");
    await expect(input).toHaveValue("");
    await expect.poll(async () => Number(await count.innerText())).toBe(total);
  });

  test("'/' keystroke focuses the search input", async ({ page }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    await page.locator("body").click({ position: { x: 1, y: 1 } });
    await page.keyboard.press("/");
    await expect(page.locator("#dxi-picker-input")).toBeFocused();
  });

  test("Escape clears search text before blurring the input", async ({ page }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    const count = page.locator("#dxi-picker-count strong");
    const total = Number(await count.innerText());

    await input.fill("trash");
    await expect(count).toHaveText("4");

    await page.keyboard.press("Escape");
    await expect(input).toBeFocused();
    await expect(input).toHaveValue("");
    await expect.poll(async () => Number(await count.innerText())).toBe(total);

    await page.keyboard.press("Escape");
    await expect(input).not.toBeFocused();
  });

  test("ArrowRight moves the active cell to the next sibling", async ({
    page,
  }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    await input.focus();
    await page.keyboard.press("ArrowRight");

    const activeIndex = await page
      .locator('#dxi-picker-grid [data-active="true"]')
      .getAttribute("data-index");
    expect(activeIndex).toBe("1");
  });

  test("ArrowLeft and ArrowUp clamp navigation at the first result", async ({
    page,
  }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    const firstCell = page.locator('#dxi-picker-grid [data-index="0"]');

    await input.focus();
    await page.keyboard.press("ArrowLeft");
    await expect(firstCell).toBeFocused();
    await expect(firstCell).toHaveAttribute("data-active", "true");

    await page.keyboard.press("ArrowUp");
    await expect(firstCell).toBeFocused();
    await expect(firstCell).toHaveAttribute("data-active", "true");
  });

  test("Enter opens the active icon from the search input", async ({ page }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    await input.fill("trash");

    const activeHref = await page
      .locator('#dxi-picker-grid [data-active="true"]')
      .getAttribute("href");
    const expectedUrl = new URL(activeHref!, DOC_URL).href;

    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(expectedUrl);
  });

  test("arrow keys keep moving after focus enters the grid", async ({ page }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    const grid = page.locator("#dxi-picker-grid");
    const trackCount = await grid.evaluate(
      (el) =>
        getComputedStyle(el)
          .gridTemplateColumns.split(" ")
          .filter((token) => token.length > 0).length,
    );

    await input.focus();
    await page.keyboard.press("ArrowRight");
    await expect(grid.locator('[data-index="1"]')).toBeFocused();

    await page.keyboard.press("ArrowRight");
    await expect(grid.locator('[data-index="2"]')).toBeFocused();

    await page.keyboard.press("ArrowDown");
    const activeIndex = await grid
      .locator('[data-active="true"]')
      .getAttribute("data-index");
    expect(activeIndex).toBe(String(trackCount + 2));
  });

  test("Enter opens the focused icon from the grid", async ({ page }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    const grid = page.locator("#dxi-picker-grid");

    await input.focus();
    await page.keyboard.press("ArrowRight");

    const focusedHref = await grid
      .locator('[data-index="1"]')
      .getAttribute("href");
    const expectedUrl = new URL(focusedHref!, DOC_URL).href;

    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(expectedUrl);
  });

  test("empty search renders the empty state without navigation", async ({
    page,
  }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const input = page.locator("#dxi-picker-input");
    await input.fill("no-icon-should-match-this-query");

    await expect(page.locator("#dxi-picker-count strong")).toHaveText("0");
    await expect(page.locator("#dxi-picker-grid .dxi-picker-cell")).toHaveCount(0);
    await expect(page.locator(".dxi-picker-empty-title")).toHaveText(
      "No icons match",
    );
    await expect(page.locator(".dxi-picker-empty-hint")).toHaveText(
      "Try a shorter or different search term.",
    );

    await page.keyboard.press("Enter");
    expect(page.url()).toBe(DOC_URL);
  });

  test("each cell links to its lucide function page", async ({ page }) => {
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);

    const hrefs = await page
      .locator("#dxi-picker-grid .dxi-picker-cell")
      .evaluateAll((cells) =>
        cells.slice(0, 8).map((el) => (el as HTMLAnchorElement).getAttribute("href")),
      );
    expect(hrefs.length).toBeGreaterThan(0);
    for (const href of hrefs) {
      expect(href).toMatch(/^lucide\/fn\.[A-Za-z0-9_]+\.html$/);
    }
  });

  test("picker renders and filters within the local latency budget", async ({
    page,
  }) => {
    const startedAt = Date.now();
    await page.goto(DOC_URL);
    await waitForInteractivePicker(page);
    expect(Date.now() - startedAt).toBeLessThan(MAX_PICKER_READY_MS);

    const result = await page.evaluate(async () => {
      const input = document.querySelector<HTMLInputElement>("#dxi-picker-input");
      const count = document.querySelector<HTMLElement>(
        "#dxi-picker-count strong",
      );
      if (!input || !count) {
        throw new Error("interactive picker controls are missing");
      }
      const started = performance.now();
      input.value = "trash";
      input.dispatchEvent(new Event("input", { bubbles: true }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      return {
        count: Number(count.textContent ?? "0"),
        ms: performance.now() - started,
      };
    });

    expect(result.count).toBe(4);
    expect(result.ms).toBeLessThan(MAX_PICKER_SEARCH_MS);
  });
});

test.describe("per-icon docs pages", () => {
  test("related icon links point to sibling lucide function pages", async ({
    page,
  }) => {
    await page.goto(TRASH_URL);

    const related = page.locator(".dioxus-icons-related-link");
    await expect(related).toHaveCount(6);
    await expect(related.first()).toContainText("Trash2");

    const hrefs = await related.evaluateAll((links) =>
      links.map((el) => (el as HTMLAnchorElement).getAttribute("href")),
    );
    expect(hrefs).not.toContain("fn.Trash.html");
    for (const href of hrefs) {
      expect(href).toMatch(/^fn\.[A-Za-z0-9_]+\.html$/);
    }

    const expectedUrl = new URL("fn.Trash2.html", TRASH_URL).href;
    await related.first().click();
    await expect(page).toHaveURL(expectedUrl);
  });

  test("widget copy writes the current RSX snippet", async ({ page }) => {
    await page.addInitScript(() => {
      const holder = window as unknown as { __dxiCopiedText: string[] };
      holder.__dxiCopiedText = [];
      const clipboard = {
        writeText: async (value: string) => {
          holder.__dxiCopiedText.push(value);
        },
      };
      try {
        Object.defineProperty(navigator, "clipboard", {
          configurable: true,
          value: clipboard,
        });
      } catch {
        (navigator as Navigator & { clipboard?: typeof clipboard }).clipboard =
          clipboard;
      }
    });

    await page.goto(TRASH_URL);

    const widget = page.locator(".dioxus-icons-widget").first();
    const code = widget.locator("[data-di-code]");
    const copy = widget.locator("[data-di-copy]");
    const expectedSnippet =
      'Trash { size: 32, stroke: "#ff0000", stroke_width: 3 }';

    await expect(widget).toBeVisible();
    await expect(widget.locator("pre[data-language='rust']")).toHaveClass(
      /dxc-system-light-github-light/,
    );
    await expect(code.locator(".a-t")).toHaveText("Trash");
    await expect(code.locator(".a-s")).toHaveText('"#000000"');
    await expect(code).toHaveText(
      'Trash { size: 24, stroke: "#000000", stroke_width: 2 }',
    );

    await widget.locator("[data-di-size]").fill("32");
    await widget.locator("[data-di-color]").fill("#ff0000");
    await widget.locator("[data-di-stroke]").fill("3");
    await expect(code).toHaveText(expectedSnippet);

    const preview = widget.locator("[data-di-preview] svg");
    await expect(preview).toHaveAttribute("width", "32");
    await expect(preview).toHaveAttribute("height", "32");
    await expect(preview).toHaveAttribute("stroke", "#ff0000");
    await expect(preview).toHaveAttribute("stroke-width", "3");

    await copy.click();
    await expect(copy).toHaveAttribute("data-copied", "true");
    await expect(widget.locator("[data-di-copy-label]")).toHaveText("Copied");

    const copiedText = await page.evaluate(
      () => (window as unknown as { __dxiCopiedText: string[] }).__dxiCopiedText,
    );
    expect(copiedText).toEqual([expectedSnippet]);
  });
});

test.describe("docs homepage picker (no JavaScript)", () => {
  test.use({ javaScriptEnabled: false });

  test("static fallback renders as a CSS grid", async ({ page }) => {
    await page.goto(DOC_URL);

    const fallback = page.locator("#dioxus-icons-static-picker");
    await expect(fallback).toBeVisible();

    const firstGrid = page.locator(".dioxus-icons-static-grid").first();
    const display = await firstGrid.evaluate(
      (el) => getComputedStyle(el).display,
    );
    expect(display).toBe("grid");

    const trackCount = await firstGrid.evaluate(
      (el) =>
        getComputedStyle(el)
          .gridTemplateColumns.split(" ")
          .filter((token) => token.length > 0).length,
    );
    expect(trackCount).toBeGreaterThan(2);

    const cellTops = await firstGrid
      .locator(".dioxus-icons-static-cell")
      .evaluateAll((cells) =>
        cells.slice(0, 16).map((el) => (el as HTMLElement).offsetTop),
      );
    const distinctTops = new Set(cellTops).size;
    expect(distinctTops).toBeLessThanOrEqual(2);

    const firstHref = await firstGrid
      .locator(".dioxus-icons-static-cell")
      .first()
      .getAttribute("href");
    expect(firstHref).toMatch(/^lucide\/fn\.[A-Za-z0-9_]+\.html$/);
  });
});
