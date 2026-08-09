import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, "../..");
const websiteRoot = join(repositoryRoot, "website");
const sourceDirectory = join(websiteRoot, "src");
const outputDirectory = join(websiteRoot, "dist");

const requiredSourceFiles = [
  "index.html",
  "404.html",
  "styles.css",
  "script.js",
  "robots.txt",
  "sitemap.xml",
  "site.webmanifest",
  "assets/apple-touch-icon.png",
  "assets/kittens-social-card.png",
];

function git(...arguments_) {
  return execFileSync("git", arguments_, {
    cwd: repositoryRoot,
    encoding: "utf8",
  }).trim();
}

function countTree(directory) {
  let bytes = 0;
  let files = 0;

  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      const child = countTree(path);
      bytes += child.bytes;
      files += child.files;
    } else if (entry.isFile()) {
      bytes += statSync(path).size;
      files += 1;
    }
  }

  return { bytes, files };
}

for (const file of requiredSourceFiles) {
  const path = join(sourceDirectory, file);
  if (!existsSync(path)) {
    throw new Error(`Missing required website source: ${relative(repositoryRoot, path)}`);
  }
}

const sourceCommit = process.env.KITTENS_SITE_SOURCE_SHA || git("rev-parse", "HEAD");
if (!/^[0-9a-f]{40}$/u.test(sourceCommit)) {
  throw new Error("KITTENS_SITE_SOURCE_SHA must be a full 40-character Git commit SHA");
}

git("rev-parse", "--verify", `${sourceCommit}^{commit}`);

const sourceDate = git("show", "-s", "--format=%cI", sourceCommit);
const sourceDateOnly = sourceDate.slice(0, 10);
const sourceShort = sourceCommit.slice(0, 8);

rmSync(outputDirectory, { recursive: true, force: true });
mkdirSync(outputDirectory, { recursive: true });
cpSync(sourceDirectory, outputDirectory, { recursive: true });

const outputAssets = join(outputDirectory, "assets");
mkdirSync(outputAssets, { recursive: true });
cpSync(join(repositoryRoot, "assets/kittens-logo.webp"), join(outputAssets, "kittens-logo.webp"));
cpSync(
  join(repositoryRoot, "assets/kittens-yarn-banner.webp"),
  join(outputAssets, "kittens-yarn-banner.webp"),
);

const replacements = new Map([
  ["__SOURCE_SHA__", sourceCommit],
  ["__SOURCE_SHORT__", sourceShort],
  ["__SOURCE_DATE__", sourceDate],
  ["__SOURCE_DATE_ONLY__", sourceDateOnly],
]);

for (const filename of ["index.html", "404.html", "sitemap.xml", "robots.txt"]) {
  const path = join(outputDirectory, filename);
  let contents = readFileSync(path, "utf8");
  for (const [placeholder, value] of replacements) {
    contents = contents.replaceAll(placeholder, value);
  }
  writeFileSync(path, contents);
}

const buildMetadata = {
  name: "kittens-public-website",
  schema_version: 1,
  site_version: "W0",
  source_commit: sourceCommit,
  source_date: sourceDate,
  source_repository: "https://github.com/Gonzih/kittens-rs",
};

writeFileSync(join(outputDirectory, "build.json"), `${JSON.stringify(buildMetadata, null, 2)}\n`);
writeFileSync(join(outputDirectory, ".nojekyll"), "");
writeFileSync(
  join(outputDirectory, "README.md"),
  `# Generated Kittens website\n\n` +
    `This branch is generated from [\`${sourceShort}\`](https://github.com/Gonzih/kittens-rs/commit/${sourceCommit}). ` +
    `Edit \`website/src/\` on \`main\`; do not edit this branch by hand.\n`,
);

for (const filename of ["index.html", "404.html", "sitemap.xml"]) {
  const contents = readFileSync(join(outputDirectory, filename), "utf8");
  if (contents.includes("__SOURCE_")) {
    throw new Error(`Unresolved source placeholder in ${filename}`);
  }
}

const totals = countTree(outputDirectory);
process.stdout.write(
  `Built ${totals.files} files (${totals.bytes} bytes) from ${sourceShort} into ${relative(repositoryRoot, outputDirectory)}\n`,
);
