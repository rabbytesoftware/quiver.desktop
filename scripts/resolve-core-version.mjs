const NIGHTLY_TAG = "nightly-latest";
const CHANNEL_PREFIXES = { stable: "stable-", beta: "beta-" };

const DATE_SERIES = /^(\d{4})-(\d{2})-(\d{2})(?:\.(\d+))?$/;
const CALENDAR_SERIES = /^\d+(?:\.\d+){0,3}$/;
const BETA_TAG = /^(\d{4}-\d{2}-\d{2}(?:\.\d+)?|\d+(?:\.\d+)*)(?:-(\d+))?$/;

/**
 * The four numbers a release version ranks by, in the order quiver.core's own
 * channel ranking (and its release-tag.sh) uses: a calendar version `26.5.1`
 * is 26.5.1.0, a date `2026-09-27.1` is 26.9.27.1. `null` for anything else,
 * including a date that is not a real one.
 */
function rankKey(version) {
  const date = DATE_SERIES.exec(version);
  if (date) {
    const [year, month, day, patch] = date.slice(1).map((part) => Number(part ?? 0));
    if (year < 2000 || year > 2099 || month < 1 || month > 12 || day < 1 || day > 31) return null;
    return [year - 2000, month, day, patch];
  }
  if (!CALENDAR_SERIES.test(version)) return null;
  return [...version.split(".").map(Number), 0, 0, 0].slice(0, 4);
}

function compareKeys(a, b) {
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

function highest(ranked) {
  return ranked.sort((a, b) => compareKeys(a.key, b.key))[ranked.length - 1].tag;
}

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
    const ranked = candidates
      .map((tag) => ({ tag, key: rankKey(tag.slice(prefix.length)) }))
      .filter((r) => r.key !== null);
    if (ranked.length === 0) {
      throw new Error(`no ${channel} release found`);
    }
    return highest(ranked);
  }

  // beta tags are `beta-<series>[-<count>]`, the series a calendar version or
  // a date ("beta-26.5-4", "beta-2026-09-27-1"). A tag of any other shape is
  // an error rather than something silently sorted in.
  const ranked = candidates.map((tag) => {
    const match = BETA_TAG.exec(tag.slice(prefix.length));
    const series = match && rankKey(match[1]);
    if (!series) {
      throw new Error(`malformed beta tag "${tag}"`);
    }
    return { tag, key: [...series, Number(match[2] ?? 0)] };
  });
  return highest(ranked);
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
