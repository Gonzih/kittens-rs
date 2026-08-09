import { createHash } from "node:crypto";
import {
  existsSync,
  readFileSync,
  readdirSync,
  statSync,
} from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, "../..");
const targetDirectory = resolve(repositoryRoot, process.argv[2] || "website/dist");
const failures = [];

function check(condition, message) {
  if (!condition) {
    failures.push(message);
  }
}

function read(relativePath) {
  const path = join(targetDirectory, relativePath);
  check(existsSync(path), `missing required artifact: ${relativePath}`);
  return existsSync(path) ? readFileSync(path, "utf8") : "";
}

function occurrences(text, pattern) {
  return [...text.matchAll(pattern)].length;
}

function linearChannel(value) {
  const normalized = value / 255;
  return normalized <= 0.04045
    ? normalized / 12.92
    : ((normalized + 0.055) / 1.055) ** 2.4;
}

function luminance(hex) {
  const channels = hex
    .slice(1)
    .match(/.{2}/gu)
    .map((channel) => linearChannel(Number.parseInt(channel, 16)));
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrast(first, second) {
  const brighter = Math.max(luminance(first), luminance(second));
  const darker = Math.min(luminance(first), luminance(second));
  return (brighter + 0.05) / (darker + 0.05);
}

function pngDimensions(relativePath) {
  const data = readFileSync(join(targetDirectory, relativePath));
  const signature = data.subarray(0, 8).toString("hex");
  check(signature === "89504e470d0a1a0a", `${relativePath} is not a valid PNG`);
  return {
    height: data.readUInt32BE(20),
    width: data.readUInt32BE(16),
  };
}

function treeDigest(directory) {
  const digest = createHash("sha256");

  function visit(current) {
    const entries = readdirSync(current, { withFileTypes: true }).sort((a, b) =>
      a.name.localeCompare(b.name),
    );
    for (const entry of entries) {
      const path = join(current, entry.name);
      const name = relative(directory, path);
      if (entry.isDirectory()) {
        visit(path);
      } else if (entry.isFile()) {
        digest.update(name);
        digest.update("\0");
        digest.update(readFileSync(path));
        digest.update("\0");
      }
    }
  }

  visit(directory);
  return digest.digest("hex");
}

const requiredArtifacts = [
  ".nojekyll",
  "404.html",
  "README.md",
  "build.json",
  "index.html",
  "robots.txt",
  "script.js",
  "site.webmanifest",
  "sitemap.xml",
  "styles.css",
  "assets/apple-touch-icon.png",
  "assets/kittens-logo.webp",
  "assets/kittens-social-card.png",
  "assets/kittens-yarn-banner.webp",
];

for (const artifact of requiredArtifacts) {
  check(existsSync(join(targetDirectory, artifact)), `missing required artifact: ${artifact}`);
}

const html = read("index.html");
const notFound = read("404.html");
const css = read("styles.css");
const javascript = read("script.js");
const robots = read("robots.txt");
const sitemap = read("sitemap.xml");
const manifestText = read("site.webmanifest");
const buildText = read("build.json");

check(/<html\s+lang="en">/u.test(html), "index.html must declare lang=en");
check(/<meta\s+name="viewport"/u.test(html), "missing responsive viewport metadata");
check(/<meta[\s\S]*?name="description"/u.test(html), "missing description metadata");
check(
  html.includes('<link rel="canonical" href="https://gonzih.github.io/kittens-rs/">'),
  "missing canonical project URL",
);
check(html.includes('property="og:image"'), "missing Open Graph social image");
check(html.includes('name="twitter:card" content="summary_large_image"'), "missing X card metadata");
check(html.includes('href="site.webmanifest"'), "missing web manifest link");
check(html.includes('<script src="script.js" defer></script>'), "website script must be deferred");
check(occurrences(html, /<h1(?:\s|>)/gu) === 1, "index.html must contain exactly one h1");
check(html.includes('<main id="main-content">'), "missing main landmark");
check(html.includes('class="skip-link" href="#main-content"'), "missing skip link");
check(occurrences(html, /<nav(?:\s|>)/gu) >= 2, "expected primary and footer navigation landmarks");
check(html.includes('aria-live="polite"'), "copy feedback must use an aria-live region");

const ids = new Set([...html.matchAll(/\sid="([^"]+)"/gu)].map((match) => match[1]));
for (const match of html.matchAll(/href="#([^"]+)"/gu)) {
  check(ids.has(match[1]), `fragment link has no target: #${match[1]}`);
}

for (const match of html.matchAll(/<(?:a|link|script|img)[^>]+(?:href|src)="([^"]+)"/gu)) {
  const url = match[1];
  if (url.startsWith("#") || url.startsWith("https://")) {
    continue;
  }
  check(!url.startsWith("/"), `root-relative local URL is not project-path safe: ${url}`);
  const withoutFragment = url.split("#", 1)[0].split("?", 1)[0];
  if (!withoutFragment || withoutFragment === "./") {
    continue;
  }
  const localPath = join(targetDirectory, withoutFragment);
  check(existsSync(localPath), `local URL does not resolve: ${url}`);
}

for (const match of html.matchAll(/<img\b[^>]*>/gu)) {
  const image = match[0];
  check(/\swidth="\d+"/u.test(image), `image is missing width: ${image.slice(0, 90)}`);
  check(/\sheight="\d+"/u.test(image), `image is missing height: ${image.slice(0, 90)}`);
  check(/\salt="[^"]*"/u.test(image), `image is missing alt: ${image.slice(0, 90)}`);
}

check(
  html.includes('loading="lazy"') && html.includes('decoding="async"'),
  "below-fold imagery must be lazy and asynchronously decoded",
);
check(
  notFound.includes('<base href="https://gonzih.github.io/kittens-rs/">'),
  "404 page must resolve relative assets from the project root",
);
check(occurrences(notFound, /<h1(?:\s|>)/gu) === 1, "404.html must contain exactly one h1");

const requiredCopy = [
  "Make async orchestration harder to get wrong.",
  "Meet kittens-code",
  "kittens-code-protocol",
  "kittens-code-core",
  "kittens-code-driver-tokio",
  "kittens-code-cli",
  "cargo install kittens-code-cli --version 0.0.1",
  "source lives on <code>kc0</code>, not deployed <code>main</code>",
  "driver topology and E1 evaluation rig remain deferred KC0 scope",
  "Op → Submission",
  "handle() → Transition",
  "Commit → Persisted",
  "DISPLAY",
  "ORCHESTRATION",
  "COGNITION",
  "ask-each",
  "Inexpressible",
  "Static detection",
  "Deterministic schedules",
  "Confidence ends where the declared vocabulary ends.",
  "not a runtime, scheduler, HAL, or sandbox",
  "formal K0 gates still reported open",
  "Direction, clearly labeled",
  "No cookies. No analytics.",
];

for (const copy of requiredCopy) {
  check(html.includes(copy), `required evidence-boundary copy is missing: ${copy}`);
}

const requiredFlagshipLinks = [
  "https://github.com/Gonzih/kittens-rs/tree/kc0",
  "https://github.com/Gonzih/kittens-rs/blob/kc0/docs/kittens-code/SPEC.md",
  "https://github.com/Gonzih/kittens-rs/blob/kc0/docs/kittens-code/RESEARCH.md",
  "https://github.com/Gonzih/kittens-rs/blob/kc0/docs/kittens-code/FRONTMATTER.md",
  "https://github.com/Gonzih/kittens-rs/blob/kc0/CHANGELOG.md#kittens-code-family-001--2026-08-09",
  "https://github.com/Gonzih/kittens-rs/tree/kc0/docs/kittens-code/research-inputs",
  "https://crates.io/crates/kittens-code-protocol",
  "https://crates.io/crates/kittens-code-core",
  "https://crates.io/crates/kittens-code-driver-tokio",
  "https://crates.io/crates/kittens-code-cli",
  "https://docs.rs/kittens-code-protocol/0.0.1/kittens_code_protocol/",
  "https://docs.rs/kittens-code-core/0.0.1/kittens_code_core/",
  "https://docs.rs/kittens-code-driver-tokio/0.0.1/kittens_code_driver_tokio/",
  "https://docs.rs/kittens-code-cli/0.0.1/kittens_code_cli/",
];

for (const url of requiredFlagshipLinks) {
  check(html.includes(`href="${url}"`), `required kittens-code link is missing: ${url}`);
}

for (const forbiddenClaim of [
  "prevents all race conditions",
  "guarantees all concurrency",
  "production-ready",
  "99.9999% coverage",
]) {
  check(!html.toLowerCase().includes(forbiddenClaim), `forbidden overclaim found: ${forbiddenClaim}`);
}

check(css.includes("@media (prefers-reduced-motion: reduce)"), "missing reduced-motion override");
check(css.includes(":focus-visible"), "missing visible keyboard focus styling");
check(css.includes("scroll-padding-top"), "sticky navigation must reserve anchor scroll space");
check(css.includes("min-height: 44px"), "interactive controls must carry the 44px target policy");

const palette = new Map();
for (const match of css.matchAll(/--([a-z][a-z-]+):\s*(#[0-9a-f]{6});/gu)) {
  palette.set(match[1], match[2]);
}

const contrastPairs = [
  ["ink", "paper", 4.5],
  ["ink-soft", "paper", 4.5],
  ["coral-deep", "coral-soft", 4.5],
  ["teal", "paper", 4.5],
  ["night-text", "night", 4.5],
  ["ink", "coral", 4.5],
];

for (const [foreground, background, minimum] of contrastPairs) {
  check(palette.has(foreground), `missing CSS palette token: --${foreground}`);
  check(palette.has(background), `missing CSS palette token: --${background}`);
  if (palette.has(foreground) && palette.has(background)) {
    const ratio = contrast(palette.get(foreground), palette.get(background));
    check(
      ratio >= minimum,
      `contrast ${foreground}/${background} is ${ratio.toFixed(2)}:1; expected ${minimum}:1`,
    );
  }
}

for (const forbiddenRuntime of [
  "fetch(",
  "XMLHttpRequest",
  "sendBeacon",
  "localStorage",
  "sessionStorage",
  "document.cookie",
]) {
  check(!javascript.includes(forbiddenRuntime), `runtime network/tracking surface found: ${forbiddenRuntime}`);
}

check(!/<form(?:\s|>)/u.test(html), "W0 must not include a form");
check(!/<(?:script|img)[^>]+src="https?:/u.test(html), "remote runtime or image dependency found");
check(!/<link[^>]+rel="stylesheet"[^>]+href="https?:/u.test(html), "remote stylesheet found");

let manifest;
try {
  manifest = JSON.parse(manifestText);
  check(manifest.start_url === "./", "manifest start_url must be project-relative");
  check(manifest.scope === "./", "manifest scope must be project-relative");
  check(Array.isArray(manifest.icons) && manifest.icons.length >= 2, "manifest icons are incomplete");
} catch (error) {
  failures.push(`invalid site.webmanifest JSON: ${error.message}`);
}

let build;
try {
  build = JSON.parse(buildText);
  check(build.schema_version === 1, "unexpected build.json schema version");
  check(build.site_version === "W0.1", "unexpected site version in build.json");
  check(/^[0-9a-f]{40}$/u.test(build.source_commit), "build.json needs a full source commit");
  check(
    build.source_repository === "https://github.com/Gonzih/kittens-rs",
    "build.json source repository is wrong",
  );
  check(html.includes(build.source_commit), "footer provenance does not match build.json");
} catch (error) {
  failures.push(`invalid build.json: ${error.message}`);
}

check(
  robots.includes("Sitemap: https://gonzih.github.io/kittens-rs/sitemap.xml"),
  "robots.txt must name the canonical sitemap",
);
check(sitemap.includes("https://gonzih.github.io/kittens-rs/"), "sitemap is missing the canonical URL");
check(!html.includes("__SOURCE_"), "index.html contains an unresolved source placeholder");
check(!sitemap.includes("__SOURCE_"), "sitemap.xml contains an unresolved source placeholder");

const socialCard = pngDimensions("assets/kittens-social-card.png");
const socialRatio = socialCard.width / socialCard.height;
check(socialCard.width >= 1200, "social card is too narrow for a large unfurl");
check(socialCard.height >= 630, "social card is too short for a large unfurl");
check(socialRatio >= 1.85 && socialRatio <= 2, `social card ratio ${socialRatio.toFixed(3)} is not landscape-safe`);

const touchIcon = pngDimensions("assets/apple-touch-icon.png");
check(touchIcon.width === 180 && touchIcon.height === 180, "apple touch icon must be 180x180");

const initialPaths = [
  "index.html",
  "styles.css",
  "script.js",
  "assets/kittens-logo.webp",
];
const initialBytes = initialPaths.reduce(
  (total, path) => total + (existsSync(join(targetDirectory, path)) ? statSync(join(targetDirectory, path)).size : 0),
  0,
);
const scrolledBytes = initialBytes +
  (existsSync(join(targetDirectory, "assets/kittens-yarn-banner.webp"))
    ? statSync(join(targetDirectory, "assets/kittens-yarn-banner.webp")).size
    : 0);
check(initialBytes < 250 * 1024, `initial page budget is ${initialBytes} bytes; expected under 256000`);
check(scrolledBytes < 500 * 1024, `full page budget is ${scrolledBytes} bytes; expected under 512000`);

const digest = treeDigest(targetDirectory);

if (failures.length > 0) {
  for (const failure of failures) {
    process.stderr.write(`FAIL: ${failure}\n`);
  }
  process.exitCode = 1;
} else {
  process.stdout.write(
    `Website checks passed (${initialBytes} initial bytes, ${scrolledBytes} scrolled bytes, sha256 ${digest.slice(0, 16)})\n`,
  );
}
