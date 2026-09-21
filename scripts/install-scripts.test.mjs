// Unit tests for the one-line installers at the repo root.
//
// install.sh is exercised for real: every test sources it with
// QUIVER_INSTALL_SOURCE_ONLY=1 (which defines its functions and runs nothing)
// and calls one function, so the shell logic under test is the shell logic
// that ships. Nothing here touches the network: there is no published
// quiver.desktop release to download yet, so the end-to-end path (download,
// mount, install) is NOT covered by these tests and is not claimed to be.
//
// install.ps1 gets static consistency checks only: no PowerShell interpreter
// exists on the machines this suite runs on.

import { describe, it, expect } from "vitest";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const installSh = path.join(repoRoot, "install.sh");
const installPs1 = path.join(repoRoot, "install.ps1");
const arrowMd = path.join(repoRoot, "ARROW.md");

/** Source install.sh, then run `code` in the same shell. */
function sourced(code, env = {}) {
  return spawnSync("sh", ["-c", `. "${installSh}"\n${code}`], {
    encoding: "utf8",
    env: { ...process.env, QUIVER_INSTALL_SOURCE_ONLY: "1", ...env },
  });
}

/** Run install.sh as a program, with arguments. */
function run(args, env = {}) {
  return spawnSync("sh", [installSh, ...args], {
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
}

// Shaped like a real GitHub /releases/latest document: the bundles
// .github/workflows/stable-release.yml actually uploads (AppImage, deb, dmg,
// msi, NSIS setup.exe), and no checksum manifest, because that workflow
// publishes none.
const release = JSON.stringify({
  tag_name: "stable-26.5",
  assets: [
    "Quiver_0.1.0_amd64.AppImage",
    "Quiver_0.1.0_amd64.deb",
    "Quiver_0.1.0_aarch64.AppImage",
    "Quiver_0.1.0_universal.dmg",
    "Quiver_0.1.0_x64_en-US.msi",
    "Quiver_0.1.0_x64-setup.exe",
  ].map((name) => ({
    name,
    browser_download_url: `https://github.com/rabbytesoftware/quiver.desktop/releases/download/stable-26.5/${name}`,
  })),
});

describe("install.sh: platform detection", () => {
  it.each([
    ["Linux", "linux"],
    ["Darwin", "macos"],
  ])("maps uname -s %s to %s", (kernel, expected) => {
    const r = sourced(`uname() { echo ${kernel}; }\ndetect_platform`);
    expect(r.status).toBe(0);
    expect(r.stdout.trim()).toBe(expected);
  });

  it("points Windows shells at install.ps1 instead of guessing", () => {
    const r = sourced(`uname() { echo MINGW64_NT-10.0; }\ndetect_platform`);
    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/install\.ps1/);
  });

  it("refuses an operating system it has no assets for", () => {
    const r = sourced(`uname() { echo FreeBSD; }\ndetect_platform`);
    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/unsupported operating system: FreeBSD/);
  });

  it.each([
    ["x86_64", "amd64"],
    ["amd64", "amd64"],
    ["aarch64", "arm64"],
    ["arm64", "arm64"],
    ["riscv64", "unknown"],
  ])("maps uname -m %s to %s", (machine, expected) => {
    const r = sourced(`uname() { echo ${machine}; }\ndetect_arch`);
    expect(r.status).toBe(0);
    expect(r.stdout.trim()).toBe(expected);
  });
});

describe("install.sh: asset selection", () => {
  const select = (platform, arch) =>
    sourced(`printf '%s' "$RELEASE_JSON" | select_asset_url ${platform} ${arch}`, {
      RELEASE_JSON: release,
    });

  it("picks the AppImage matching the architecture on Linux", () => {
    expect(select("linux", "amd64").stdout.trim()).toMatch(/Quiver_0\.1\.0_amd64\.AppImage$/);
    expect(select("linux", "arm64").stdout.trim()).toMatch(/Quiver_0\.1\.0_aarch64\.AppImage$/);
  });

  it("never picks the .deb, which needs root and a Debian-family distro", () => {
    expect(select("linux", "amd64").stdout).not.toMatch(/\.deb/);
  });

  it("picks the .dmg on macOS", () => {
    expect(select("macos", "arm64").stdout.trim()).toMatch(/Quiver_0\.1\.0_universal\.dmg$/);
  });

  it("falls back to the only asset of its kind when the arch does not appear in the name", () => {
    // The universal dmg names no architecture, so arch narrowing must not
    // filter it away.
    expect(select("macos", "amd64").stdout.trim()).toMatch(/universal\.dmg$/);
  });

  it("selects nothing by architecture when uname -m was unrecognised", () => {
    const r = select("macos", "unknown");
    expect(r.status).toBe(0);
    expect(r.stdout.trim()).toMatch(/universal\.dmg$/);
  });

  it("fails loudly when the release carries no asset of the right kind", () => {
    const r = sourced(`printf '%s' "$RELEASE_JSON" | select_asset_url linux amd64`, {
      RELEASE_JSON: JSON.stringify({
        assets: [{ name: "notes.txt", browser_download_url: "https://example.invalid/notes.txt" }],
      }),
    });
    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/publishes no .*AppImage/);
  });

  it("reads every asset url out of the API document", () => {
    const r = sourced(`printf '%s' "$RELEASE_JSON" | list_asset_urls | wc -l`, {
      RELEASE_JSON: release,
    });
    expect(Number(r.stdout.trim())).toBe(6);
  });
});

describe("install.sh: checksum handling", () => {
  it("finds no checksum manifest in a release that publishes none", () => {
    const r = sourced(`printf '%s' "$RELEASE_JSON" | select_checksums_url`, {
      RELEASE_JSON: release,
    });
    expect(r.status).toBe(0);
    expect(r.stdout.trim()).toBe("");
  });

  it("finds a checksum manifest when one is published", () => {
    const withSums = JSON.stringify({
      assets: [
        { name: "Quiver_0.1.0_amd64.AppImage", browser_download_url: "https://example.invalid/a.AppImage" },
        { name: "checksums.txt", browser_download_url: "https://example.invalid/checksums.txt" },
      ],
    });
    const r = sourced(`printf '%s' "$RELEASE_JSON" | select_checksums_url`, { RELEASE_JSON: withSums });
    expect(r.stdout.trim()).toBe("https://example.invalid/checksums.txt");
  });

  it.each([
    ["text form", "abc123  Quiver.AppImage", "abc123"],
    ["binary form", "abc123 *Quiver.AppImage", "abc123"],
  ])("reads the digest out of sha256sum %s", (_label, line, expected) => {
    const r = sourced(`expected_sum "$MANIFEST" Quiver.AppImage`, {
      MANIFEST: `deadbeef  other.file\n${line}\n`,
    });
    expect(r.stdout.trim()).toBe(expected);
  });

  it("reports no digest for a file the manifest does not list", () => {
    const r = sourced(`expected_sum "$MANIFEST" Quiver.AppImage`, { MANIFEST: "deadbeef  other.file\n" });
    expect(r.stdout.trim()).toBe("");
  });

  it("warns rather than pretending a download was verified", () => {
    const r = sourced(`verify_checksum /dev/null "" ""`);
    expect(r.status).toBe(0);
    expect(r.stderr).toMatch(/could not be verified beyond TLS/);
  });

  // GitHub records a sha256 per asset on the releases API itself. That is
  // what verifies a quiver.desktop download today: stable-release.yml
  // publishes no checksum manifest, so without this there is nothing to check
  // against at all.
  it("reads the digest GitHub recorded for the asset it picked", () => {
    const sum = "a".repeat(64);
    const withDigest = JSON.stringify({
      assets: [
        {
          name: "Quiver_0.1.0_amd64.AppImage",
          browser_download_url: "https://example.invalid/a.AppImage",
          digest: `sha256:${sum}`,
        },
        {
          name: "Quiver_0.1.0_aarch64.AppImage",
          browser_download_url: "https://example.invalid/b.AppImage",
          digest: `sha256:${"b".repeat(64)}`,
        },
      ],
    });
    const r = sourced(`printf '%s' "$RELEASE_JSON" | asset_digest https://example.invalid/a.AppImage`, {
      RELEASE_JSON: withDigest,
    });
    expect(r.stdout.trim()).toBe(sum);
  });

  it("reports no digest for an asset that predates GitHub recording one", () => {
    const noDigest = JSON.stringify({
      assets: [
        {
          name: "Quiver_0.1.0_amd64.AppImage",
          browser_download_url: "https://example.invalid/a.AppImage",
          digest: null,
        },
      ],
    });
    const r = sourced(`printf '%s' "$RELEASE_JSON" | asset_digest https://example.invalid/a.AppImage`, {
      RELEASE_JSON: noDigest,
    });
    expect(r.stdout.trim()).toBe("");
  });

  // The real api.github.com PRETTY-PRINTS, and every asset object carries a
  // nested "uploader" object. An earlier draft split the document on "{",
  // which cuts an asset in half at that nested brace and only worked while
  // the field order happened to cooperate. This is the real shape, pretty
  // printed, with an asset that has no digest sitting between two that do --
  // the arrangement that catches a pairing that slides by one.
  it("reads the right digest out of the document shape GitHub actually sends", () => {
    const uploader = { login: "github-actions[bot]", id: 41898282, type: "Bot" };
    const realShape = JSON.stringify(
      {
        tag_name: "stable-26.5.1",
        assets: [
          {
            name: "a.AppImage",
            uploader,
            content_type: "application/octet-stream",
            size: 10,
            digest: `sha256:${"1".repeat(64)}`,
            browser_download_url: "https://example.invalid/a.AppImage",
          },
          {
            name: "b.AppImage",
            uploader,
            content_type: "application/octet-stream",
            size: 20,
            digest: null,
            browser_download_url: "https://example.invalid/b.AppImage",
          },
          {
            name: "c.AppImage",
            uploader,
            content_type: "application/octet-stream",
            size: 30,
            digest: `sha256:${"3".repeat(64)}`,
            browser_download_url: "https://example.invalid/c.AppImage",
          },
        ],
      },
      null,
      2
    );

    const digestOf = (url) =>
      sourced(`printf '%s' "$RELEASE_JSON" | asset_digest ${url}`, { RELEASE_JSON: realShape }).stdout.trim();

    expect(digestOf("https://example.invalid/a.AppImage")).toBe("1".repeat(64));
    // Must be empty, not a.AppImage's digest slid forward by one.
    expect(digestOf("https://example.invalid/b.AppImage")).toBe("");
    expect(digestOf("https://example.invalid/c.AppImage")).toBe("3".repeat(64));
  });

  it("ignores a digest under an algorithm the fetch step cannot check", () => {
    const sha512 = JSON.stringify({
      assets: [
        {
          name: "Quiver_0.1.0_amd64.AppImage",
          browser_download_url: "https://example.invalid/a.AppImage",
          digest: "sha512:" + "c".repeat(128),
        },
      ],
    });
    const r = sourced(`printf '%s' "$RELEASE_JSON" | asset_digest https://example.invalid/a.AppImage`, {
      RELEASE_JSON: sha512,
    });
    expect(r.stdout.trim()).toBe("");
  });

  it("verifies against the digest without reading any manifest", () => {
    const dir = mkdtempSync(path.join(tmpdir(), "quiver-digest-test-"));
    const file = path.join(dir, "Quiver.AppImage");
    writeFileSync(file, "payload\n");
    const digest = createHash("sha256").update(readFileSync(file)).digest("hex");

    // An empty manifest URL: if the digest were not being used, this would
    // warn instead of verifying.
    const r = sourced(`verify_checksum "$FILE" "" "$DIGEST"`, { FILE: file, DIGEST: digest });
    expect(r.status).toBe(0);
    expect(r.stderr).toMatch(/Checksum verified/);
    rmSync(dir, { recursive: true, force: true });
  });

  it("refuses a file whose digest does not match, and removes it", () => {
    const dir = mkdtempSync(path.join(tmpdir(), "quiver-digest-test-"));
    const file = path.join(dir, "Quiver.AppImage");
    writeFileSync(file, "payload\n");

    const r = sourced(`verify_checksum "$FILE" "" "$DIGEST"`, { FILE: file, DIGEST: "0".repeat(64) });
    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/checksum mismatch/);
    expect(existsSync(file)).toBe(false);
    rmSync(dir, { recursive: true, force: true });
  });
});

describe("install.sh: argument handling", () => {
  it("prints usage and exits clean for --help", () => {
    const r = run(["--help"]);
    expect(r.status).toBe(0);
    expect(r.stderr).toMatch(/USAGE:/);
  });

  it("rejects an unknown option", () => {
    const r = run(["--frobnicate"]);
    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/unknown option: --frobnicate/);
  });

  it("rejects --tag with no value", () => {
    const r = run(["--tag"]);
    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/--tag requires a value/);
  });

  it("asks GitHub for the latest release by default, and for a tag when given one", () => {
    expect(sourced("release_url").stdout.trim()).toBe(
      "https://api.github.com/repos/rabbytesoftware/quiver.desktop/releases/latest"
    );
    expect(sourced("release_url", { QUIVER_TAG: "stable-26.5" }).stdout.trim()).toBe(
      "https://api.github.com/repos/rabbytesoftware/quiver.desktop/releases/tags/stable-26.5"
    );
  });
});

// The install location is the contract between this script and the manifest.
// quiver.core's preinstalled probe looks in exactly one place on Linux; if the
// script and ARROW.md ever disagree about where that is, a script-installed app
// silently stops being detected.
describe("install.sh agrees with ARROW.md about where things go", () => {
  const manifest = readFileSync(arrowMd, "utf8");

  it("installs the AppImage where QUIVER_DESKTOP_APPIMAGE_PATH points", () => {
    const declared = manifest.match(
      /- name: "QUIVER_DESKTOP_APPIMAGE_PATH"[\s\S]*?\n {4}default: "([^"]+)"/
    );
    expect(declared, "ARROW.md must declare QUIVER_DESKTOP_APPIMAGE_PATH with a default").not.toBeNull();

    const env = { HOME: "/home/tester", XDG_DATA_HOME: "" };
    // The manifest's default is expanded by the same shell that would expand
    // it inside a run step, so this compares the resolved paths, not the text.
    const fromManifest = spawnSync("sh", ["-c", `printf '%s' "${declared[1]}"`], {
      encoding: "utf8",
      env: { HOME: env.HOME },
    });
    const fromScript = sourced("appimage_path", { HOME: env.HOME, XDG_DATA_HOME: "" });

    expect(fromScript.stdout.trim()).toBe("/home/tester/.local/share/Quiver/Quiver.AppImage");
    expect(fromManifest.stdout.trim()).toBe(fromScript.stdout.trim());
  });

  it("honours XDG_DATA_HOME in both", () => {
    const fromScript = sourced("appimage_path", { HOME: "/home/tester", XDG_DATA_HOME: "/data" });
    expect(fromScript.stdout.trim()).toBe("/data/Quiver/Quiver.AppImage");
  });

  it("looks for the AppImage it installs in the preinstalled probe", () => {
    const detect = manifest.match(/- name: "QUIVER_DESKTOP_DETECT_COMMAND"[\s\S]*?\n {4}default: '([^']+)'/);
    expect(detect).not.toBeNull();
    expect(detect[1]).toContain("/Quiver/Quiver.AppImage");
    expect(detect[1]).toContain("command -v quiverdesktop");
  });
});

// The whole of main(), with only the two functions that reach the network
// replaced by local fixtures, which is the shell equivalent of standing an httptest
// server in front of the release host. Everything else is the real thing:
// argument handling, platform detection, asset selection, the SHA-256 check
// and the Linux placement.
//
// uname is stubbed to Linux so this runs identically on a macOS and a Linux
// machine, and so that it never touches /Applications.
describe("install.sh: end to end with the release host stubbed", () => {
  function fixtures(assetBody) {
    const dir = mkdtempSync(path.join(tmpdir(), "quiver-install-test-"));
    const asset = path.join(dir, "Quiver_0.1.0_amd64.AppImage");
    writeFileSync(asset, assetBody);
    const json = path.join(dir, "release.json");
    writeFileSync(
      json,
      JSON.stringify({
        assets: [
          { name: "Quiver_0.1.0_amd64.AppImage", browser_download_url: "https://example.invalid/Quiver_0.1.0_amd64.AppImage" },
          { name: "Quiver_0.1.0_amd64.deb", browser_download_url: "https://example.invalid/Quiver_0.1.0_amd64.deb" },
          { name: "checksums.txt", browser_download_url: "https://example.invalid/checksums.txt" },
        ],
      })
    );
    return { dir, asset, json, sums: path.join(dir, "checksums.txt") };
  }

  const stubs = `
uname() { if [ "\${1:-}" = "-m" ]; then echo x86_64; else echo Linux; fi; }
http_get() {
  case "$1" in
    *releases/latest) cat "$FIXTURE_JSON" ;;
    *checksums.txt) cat "$FIXTURE_SUMS" ;;
    *) printf 'unexpected http_get %s\\n' "$1" >&2; return 1 ;;
  esac
}
http_download() { cp "$FIXTURE_ASSET" "$2"; }
`;

  it("downloads, verifies and places the AppImage", () => {
    const f = fixtures("this is not really an AppImage\n");
    const digest = createHash("sha256").update(readFileSync(f.asset)).digest("hex");
    writeFileSync(f.sums, `${digest}  Quiver_0.1.0_amd64.AppImage\n`);
    const home = path.join(f.dir, "home");

    const r = sourced(`${stubs}\nmain`, {
      HOME: home,
      XDG_DATA_HOME: "",
      FIXTURE_JSON: f.json,
      FIXTURE_SUMS: f.sums,
      FIXTURE_ASSET: f.asset,
    });

    expect(r.stderr).toMatch(/Checksum verified/);
    expect(r.status).toBe(0);
    const installed = path.join(home, ".local/share/Quiver/Quiver.AppImage");
    expect(existsSync(installed), `expected ${installed} to exist. stderr:\n${r.stderr}`).toBe(true);
    rmSync(f.dir, { recursive: true, force: true });
  });

  it("refuses to install an asset whose checksum does not match", () => {
    const f = fixtures("this is not really an AppImage\n");
    writeFileSync(f.sums, `${"0".repeat(64)}  Quiver_0.1.0_amd64.AppImage\n`);
    const home = path.join(f.dir, "home");

    const r = sourced(`${stubs}\nmain`, {
      HOME: home,
      XDG_DATA_HOME: "",
      FIXTURE_JSON: f.json,
      FIXTURE_SUMS: f.sums,
      FIXTURE_ASSET: f.asset,
    });

    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/checksum mismatch/);
    expect(existsSync(path.join(home, ".local/share/Quiver/Quiver.AppImage"))).toBe(false);
    rmSync(f.dir, { recursive: true, force: true });
  });

  it("installs, with a warning, when the release publishes no checksum manifest", () => {
    const f = fixtures("this is not really an AppImage\n");
    writeFileSync(
      f.json,
      JSON.stringify({
        assets: [
          { name: "Quiver_0.1.0_amd64.AppImage", browser_download_url: "https://example.invalid/Quiver_0.1.0_amd64.AppImage" },
        ],
      })
    );
    const home = path.join(f.dir, "home");

    const r = sourced(`${stubs}\nmain`, {
      HOME: home,
      XDG_DATA_HOME: "",
      FIXTURE_JSON: f.json,
      FIXTURE_SUMS: f.sums,
      FIXTURE_ASSET: f.asset,
    });

    expect(r.status).toBe(0);
    expect(r.stderr).toMatch(/could not be verified beyond TLS/);
    expect(existsSync(path.join(home, ".local/share/Quiver/Quiver.AppImage"))).toBe(true);
    rmSync(f.dir, { recursive: true, force: true });
  });

  // The path a real release takes today: no manifest, but a digest on the
  // asset itself, and no request for a manifest that is not there.
  it("verifies off the asset digest when the release publishes no manifest", () => {
    const f = fixtures("this is not really an AppImage\n");
    const digest = createHash("sha256").update(readFileSync(f.asset)).digest("hex");
    writeFileSync(
      f.json,
      JSON.stringify({
        assets: [
          {
            name: "Quiver_0.1.0_amd64.AppImage",
            browser_download_url: "https://example.invalid/Quiver_0.1.0_amd64.AppImage",
            digest: `sha256:${digest}`,
          },
        ],
      })
    );
    const home = path.join(f.dir, "home");

    const r = sourced(`${stubs}\nmain`, {
      HOME: home,
      XDG_DATA_HOME: "",
      FIXTURE_JSON: f.json,
      FIXTURE_SUMS: f.sums,
      FIXTURE_ASSET: f.asset,
    });

    expect(r.stderr).toMatch(/Checksum verified/);
    expect(r.stderr).not.toMatch(/could not be verified/);
    expect(r.status).toBe(0);
    expect(existsSync(path.join(home, ".local/share/Quiver/Quiver.AppImage"))).toBe(true);
    rmSync(f.dir, { recursive: true, force: true });
  });
});

describe("install.ps1", () => {
  const ps1 = readFileSync(installPs1, "utf8");
  const manifest = readFileSync(arrowMd, "utf8");

  it("selects the NSIS setup executable and never the per-machine .msi", () => {
    expect(ps1).toContain("-setup\\.exe$");
    expect(ps1).not.toMatch(/\.msi'\s*\}/);
  });

  it("agrees with ARROW.md on the currentUser install directory", () => {
    expect(ps1).toContain("Join-Path $env:LOCALAPPDATA 'Quiver'");
    expect(manifest).toContain("%LOCALAPPDATA%\\Quiver\\quiverdesktop.exe");
  });

  it("does not use syntax Windows PowerShell 5.1 lacks", () => {
    // $IsWindows and ?? are PowerShell 6+. The one-liner most often lands in
    // 5.1, where the first is silently $null and the second is a parse error.
    // Comments are stripped first: both are named there, deliberately, to say
    // why they are avoided.
    const code = ps1
      .replace(/<#[\s\S]*?#>/g, "")
      .split("\n")
      .filter((line) => !/^\s*#/.test(line))
      .join("\n");
    expect(code).not.toContain("$IsWindows");
    expect(code).not.toContain("??");
  });
});
