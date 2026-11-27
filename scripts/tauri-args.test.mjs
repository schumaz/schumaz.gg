import { describe, expect, it } from "vitest";
import { withMainBinaryArg } from "./tauri-args.mjs";

describe("withMainBinaryArg", () => {
  it("appends runner --bin for a bare tauri dev", () => {
    expect(withMainBinaryArg(["dev"])).toEqual(["dev", "--", "--bin", "schumazgg"]);
  });

  it("injects --bin into existing runner args while preserving app args", () => {
    expect(withMainBinaryArg(["dev", "--", "--release", "--", "--autostart"])).toEqual([
      "dev",
      "--",
      "--bin",
      "schumazgg",
      "--release",
      "--",
      "--autostart",
    ]);
  });

  it("keeps an explicit runner --bin and leaves non-dev commands untouched", () => {
    expect(withMainBinaryArg(["dev", "--", "--bin", "other"])).toEqual([
      "dev",
      "--",
      "--bin",
      "other",
    ]);
    expect(withMainBinaryArg(["build", "--features", "portable"])).toEqual([
      "build",
      "--features",
      "portable",
    ]);
  });
});
