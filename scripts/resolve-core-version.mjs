import semver from "semver";

const STABLE_PREFIX = "stable-";

/**
 * Picks the highest stable-<semver> tag satisfying a semver range.
 * Non-stable channels (beta-, hotfix-) are never eligible — Desktop's build
 * only ever bundles a stable release as its bootstrap seed.
 */
export function resolveTag(tags, constraint) {
  const candidates = tags
    .filter((t) => t.startsWith(STABLE_PREFIX))
    .map((t) => t.slice(STABLE_PREFIX.length))
    .filter((v) => semver.valid(semver.coerce(v)));

  const best = semver.maxSatisfying(candidates, constraint, { loose: true });
  if (!best) {
    throw new Error(`no stable release found satisfying "${constraint}"`);
  }
  return `${STABLE_PREFIX}${best}`;
}

async function main() {
  const constraint = process.argv[2];
  if (!constraint) {
    console.error("usage: resolve-core-version.mjs <constraint>");
    process.exit(1);
  }

  const { spawnSync } = await import("child_process");
  const proc = spawnSync("gh", [
    "release", "list",
    "--repo", "rabbytesoftware/quiver.core",
    "--json", "tagName",
    "-L", "100",
  ]);

  if (proc.error) {
    console.error("gh not found — install the GitHub CLI (https://cli.github.com)");
    process.exit(1);
  }

  if (proc.status !== 0) {
    if (proc.stderr) {
      console.error(proc.stderr.toString());
    } else {
      console.error("gh command failed with unknown error");
    }
    process.exit(1);
  }

  try {
    const releases = JSON.parse(proc.stdout.toString());
    const tags = releases.map((r) => r.tagName);
    console.log(resolveTag(tags, constraint));
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }
}

if (import.meta.main) {
  await main();
}
