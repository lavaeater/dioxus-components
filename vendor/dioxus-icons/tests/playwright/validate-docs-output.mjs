#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(HERE, "../..");
const DOC_ROOT = path.join(REPO_ROOT, "target/doc/dioxus_icons");
const INDEX_HTML = path.join(DOC_ROOT, "index.html");
const TRASH_HTML = path.join(DOC_ROOT, "lucide/fn.Trash.html");

const MAX_TOTAL_BYTES = Number(
  process.env.DXI_MAX_DOC_BYTES ?? 225 * 1024 * 1024,
);
const MAX_INDEX_BYTES = Number(
  process.env.DXI_MAX_INDEX_BYTES ?? 3 * 1024 * 1024,
);
const MAX_NON_INDEX_HTML_BYTES = Number(
  process.env.DXI_MAX_NON_INDEX_HTML_BYTES ?? 768 * 1024,
);
const MAX_ICON_PAGE_BYTES = Number(
  process.env.DXI_MAX_ICON_PAGE_BYTES ?? 96 * 1024,
);

const errors = [];

function check(condition, message) {
  if (!condition) errors.push(message);
}

function readText(file) {
  try {
    return fs.readFileSync(file, "utf8");
  } catch (error) {
    errors.push(`missing generated docs file: ${path.relative(REPO_ROOT, file)}`);
    return "";
  }
}

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(fullPath, out);
    } else if (entry.isFile()) {
      out.push(fullPath);
    }
  }
  return out;
}

function formatBytes(bytes) {
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

function extractManifestText(html) {
  const match = html.match(
    /<script\b(?=[^>]*\bid="__icon_manifest__")(?=[^>]*\btype="application\/json")[^>]*>([\s\S]*?)<\/script>/,
  );
  if (!match) {
    errors.push("missing __icon_manifest__ application/json script");
    return "";
  }
  return match[1];
}

if (!fs.existsSync(INDEX_HTML)) {
  console.error(
    "Generated docs are missing. Build them first with:\n" +
      '  RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p dioxus-icons',
  );
  process.exit(1);
}

const files = walk(DOC_ROOT);
const sizes = files.map((file) => [file, fs.statSync(file).size]);
const totalBytes = sizes.reduce((sum, [, size]) => sum + size, 0);
const htmlSizes = sizes.filter(([file]) => file.endsWith(".html"));
const indexBytes = fs.statSync(INDEX_HTML).size;
const trashBytes = fs.existsSync(TRASH_HTML) ? fs.statSync(TRASH_HTML).size : 0;
const largestNonIndexHtml = htmlSizes
  .filter(([file]) => file !== INDEX_HTML)
  .sort((a, b) => b[1] - a[1])[0] ?? ["", 0];

check(
  totalBytes <= MAX_TOTAL_BYTES,
  `generated docs are ${formatBytes(totalBytes)}, above ${formatBytes(MAX_TOTAL_BYTES)}`,
);
check(
  indexBytes <= MAX_INDEX_BYTES,
  `crate index is ${formatBytes(indexBytes)}, above ${formatBytes(MAX_INDEX_BYTES)}`,
);
check(
  largestNonIndexHtml[1] <= MAX_NON_INDEX_HTML_BYTES,
  `largest non-index HTML page is ${path.relative(REPO_ROOT, largestNonIndexHtml[0])} ` +
    `at ${formatBytes(largestNonIndexHtml[1])}, above ${formatBytes(MAX_NON_INDEX_HTML_BYTES)}`,
);
check(
  trashBytes <= MAX_ICON_PAGE_BYTES,
  `representative icon page is ${formatBytes(trashBytes)}, above ${formatBytes(MAX_ICON_PAGE_BYTES)}`,
);

const indexHtml = readText(INDEX_HTML);
const trashHtml = readText(TRASH_HTML);
const manifestText = extractManifestText(indexHtml);
let manifest = { icons: [] };
if (manifestText) {
  check(
    !manifestText.includes("<svg") && !manifestText.includes("</svg"),
    "manifest JSON contains raw SVG tags; expected rustdoc-safe unicode escapes",
  );
  check(
    manifestText.includes("\\u003csvg"),
    "manifest JSON does not contain escaped SVG payloads",
  );
  check(
    !manifestText.toLowerCase().includes("</script"),
    "manifest JSON contains a raw closing script sequence",
  );
  try {
    manifest = JSON.parse(manifestText);
  } catch (error) {
    errors.push(`manifest JSON does not parse: ${error.message}`);
  }
}

check(Array.isArray(manifest.icons), "manifest icons field is not an array");
check(
  Array.isArray(manifest.icons) && manifest.icons.length > 1_000,
  `manifest icon count is too small: ${manifest.icons?.length ?? "missing"}`,
);
check(
  manifest.icons.every(
    (icon) =>
      typeof icon.name === "string" &&
      typeof icon.svg === "string" &&
      icon.svg.startsWith("<svg"),
  ),
  "parsed manifest icons are missing names or SVG payloads",
);
check(indexHtml.includes('id="dioxus-icons-static-picker"'), "missing no-JS static picker");
check(indexHtml.includes('id="dxi-picker"'), "missing interactive picker root");
check(indexHtml.includes('data-dxi-picker'), "missing interactive picker data hook");

check(trashHtml.includes('class="dioxus-icons-widget"'), "missing per-icon widget");
check(
  trashHtml.includes('data-svg="&lt;svg'),
  "per-icon widget data-svg is not HTML-escaped",
);
check(
  !trashHtml.includes('data-svg="<svg'),
  "per-icon widget data-svg contains a raw SVG attribute value",
);
check(trashHtml.includes("navigator.clipboard"), "missing clipboard copy path");
check(trashHtml.includes('document.execCommand("copy")'), "missing textarea copy fallback");
check(
  trashHtml.includes('class="dioxus-icons-related-link"'),
  "missing related-icon links",
);

if (errors.length) {
  console.error("docs-output validation failed:");
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log(
  [
    "docs-output:",
    `${files.length} files`,
    `${htmlSizes.length} html`,
    `${formatBytes(totalBytes)} logical`,
    `index ${formatBytes(indexBytes)}`,
    `largest non-index ${formatBytes(largestNonIndexHtml[1])}`,
  ].join(" "),
);
