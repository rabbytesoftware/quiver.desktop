const PATTERN = /Built against quiver\.core `([^`]+)`/;

export function extractCoreVersion(notes) {
  const match = PATTERN.exec(notes);
  if (!match) {
    throw new Error('no "Built against quiver.core `...`" line found in release notes');
  }
  return match[1];
}

async function main() {
  const tag = process.argv[2];
  if (!tag) {
    console.error("usage: extract-core-version.mjs <desktop-release-tag>");
    process.exit(1);
  }

  const { spawnSync } = await import("child_process");
  const proc = spawnSync("gh", [
    "release", "view", tag,
    "--repo", "rabbytesoftware/quiver.desktop",
    "--json", "body",
    "--jq", ".body",
  ]);

  if (proc.error) {
    console.error("gh not found — install the GitHub CLI (https://cli.github.com)");
    process.exit(1);
  }

  if (proc.status !== 0) {
    console.error(proc.stderr ? proc.stderr.toString() : "gh command failed with unknown error");
    process.exit(1);
  }

  try {
    console.log(extractCoreVersion(proc.stdout.toString()));
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }
}

if (import.meta.main) {
  await main();
}
