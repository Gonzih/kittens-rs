import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, "../..");
const buildScript = join(scriptDirectory, "build.mjs");
const outputDirectory = join(repositoryRoot, "website/dist");
const sourceCommit = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: repositoryRoot,
  encoding: "utf8",
}).trim();

function digestTree(directory) {
  const digest = createHash("sha256");

  function visit(current) {
    const entries = readdirSync(current, { withFileTypes: true }).sort((a, b) =>
      a.name.localeCompare(b.name),
    );
    for (const entry of entries) {
      const path = join(current, entry.name);
      if (entry.isDirectory()) {
        visit(path);
      } else if (entry.isFile()) {
        digest.update(relative(directory, path));
        digest.update("\0");
        digest.update(readFileSync(path));
        digest.update("\0");
      }
    }
  }

  visit(directory);
  return digest.digest("hex");
}

function build() {
  execFileSync(process.execPath, [buildScript], {
    cwd: repositoryRoot,
    env: { ...process.env, KITTENS_SITE_SOURCE_SHA: sourceCommit },
    stdio: "pipe",
  });
  return digestTree(outputDirectory);
}

const first = build();
const second = build();

if (first !== second) {
  throw new Error(`Website build is not reproducible: ${first} != ${second}`);
}

process.stdout.write(`Website build reproduced byte-for-byte (${first})\n`);
