/** Desktop shell binary inside the `src-tauri/` Cargo workspace. */
export const MAIN_BINARY_NAME = "schumazgg";

/**
 * Ensures `tauri dev` always runs the desktop shell binary.
 *
 * The `src-tauri/` workspace exposes multiple binaries (`schumazgg` and the
 * zero-Tauri `export_bindings` exporter), so a bare `cargo run` cannot pick one.
 * Runner arguments follow the first `--` (`tauri dev -- [runnerArgs] -- [appArgs]`).
 */
export function withMainBinaryArg(args, binaryName = MAIN_BINARY_NAME) {
  if (args[0] !== "dev") {
    return [...args];
  }

  const runnerStart = args.indexOf("--");
  if (runnerStart === -1) {
    return [...args, "--", "--bin", binaryName];
  }

  const appArgsStart = args.indexOf("--", runnerStart + 1);
  const runnerArgs = args.slice(
    runnerStart + 1,
    appArgsStart === -1 ? args.length : appArgsStart
  );
  if (runnerArgs.some((arg) => arg === "--bin" || arg.startsWith("--bin="))) {
    return [...args];
  }

  return [
    ...args.slice(0, runnerStart + 1),
    "--bin",
    binaryName,
    ...args.slice(runnerStart + 1),
  ];
}
