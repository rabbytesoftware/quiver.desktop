import semver from "semver";

const NIGHTLY_TAG = "nightly-latest";
const CHANNEL_PREFIXES = { stable: "stable-", beta: "beta-" };

/**
 * Picks the highest release tag on a quiver.core channel. `stable` and
 * `beta` are open ranges — no compatibility constraint restricts them any
 * more, only "whatever the channel currently calls newest". `nightly`
 * isn't a range at all, just a single rolling pointer tag.
 */
export function resolveChannel(tags, channel) {
  if (channel === "nightly") {
    if (!tags.includes(NIGHTLY_TAG)) {
      throw new Error(`nightly tag "${NIGHTLY_TAG}" not found`);
    }
    return NIGHTLY_TAG;
  }

  const prefix = CHANNEL_PREFIXES[channel];
  if (!prefix) {
    throw new Error(`unknown channel "${channel}"`);
  }

  const candidates = tags.filter((t) => t.startsWith(prefix));
  if (candidates.length === 0) {
    throw new Error(`no ${channel} release found`);
  }

  if (channel === "stable") {
    const versions = candidates
      .map((t) => t.slice(prefix.length))
      .filter((v) => semver.valid(semver.coerce(v)))
      .sort((a, b) => semver.compare(semver.coerce(a), semver.coerce(b)));
    if (versions.length === 0) {
      throw new Error(`no ${channel} release found`);
    }
    return `${prefix}${versions[versions.length - 1]}`;
  }

  // beta tags are `beta-<series>[-<count>]` (e.g. "beta-26.5", "beta-26.5-4")
  // — not valid semver on their own, so series and count are compared
  // separately rather than parsed as one semver string. Validate the shape
  // up front: an unvalidated tag (e.g. "beta-abc") would parse to NaN
  // components, and Array.prototype.sort treats a NaN comparator result as
  // "unordered", silently corrupting the sort instead of erroring.
  const BETA_TAG_SHAPE = /^\d+(\.\d+)*(-\d+)?$/;
  const parsed = candidates.map((tag) => {
    const rest = tag.slice(prefix.length);
    if (!BETA_TAG_SHAPE.test(rest)) {
      throw new Error(`malformed beta tag "${tag}"`);
    }
    const [series, count] = rest.split("-");
    return { tag, series: series.split(".").map(Number), count: count ? Number(count) : 0 };
  });
  parsed.sort((a, b) => {
    for (let i = 0; i < Math.max(a.series.length, b.series.length); i++) {
      const diff = (a.series[i] ?? 0) - (b.series[i] ?? 0);
      if (diff !== 0) return diff;
    }
    return a.count - b.count;
  });
  return parsed[parsed.length - 1].tag;
}

async function main() {
  const channel = process.argv[2];
  if (!channel) {
    console.error("usage: resolve-core-version.mjs <channel>");
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
    console.error(proc.stderr ? proc.stderr.toString() : "gh command failed with unknown error");
    process.exit(1);
  }

  try {
    const releases = JSON.parse(proc.stdout.toString());
    const tags = releases.map((r) => r.tagName);
    console.log(resolveChannel(tags, channel));
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }
}

if (import.meta.main) {
  await main();
}
