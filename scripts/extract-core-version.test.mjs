import { describe, it, expect } from "vitest";
import { extractCoreVersion } from "./extract-core-version.mjs";

describe("extractCoreVersion", () => {
  it("extracts the tag from a standard release notes footer", () => {
    const notes = "## Changes since stable-26.5.0\n- fix: something (Mateo)\n\nBuilt against quiver.core `stable-26.5.1`.\n";
    expect(extractCoreVersion(notes)).toBe("stable-26.5.1");
  });

  it("throws when no core-version line is present", () => {
    expect(() => extractCoreVersion("## Initial Release\n")).toThrow(/no "Built against/);
  });
});
