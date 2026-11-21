export { aramIpc } from "./aram.ipc";
export { arenaIpc } from "./arena.ipc";
export { assetsIpc, isTauriRuntime, SCHUMAZ_ASSET_PROTOCOL } from "./assets.ipc";
export { buildIpc } from "./build.ipc";
export { diagnosticsIpc, type DiagnosticSummary } from "./diagnostics.ipc";
export { injectorIpc } from "./injector.ipc";
export { lcuIpc, type UnlistenFn } from "./lcu.ipc";
export { profileIpc } from "./profile.ipc";
export { rankedIpc } from "./ranked.ipc";
export { settingsIpc, type AutostartStatus } from "./settings.ipc";
export {
  updaterIpc,
  type AppDistributionInfo,
  type UpdateProbeResult,
  type UpdateProbeStatus,
  type UpdaterUpdateHandle,
} from "./updater.ipc";
export { windowIpc } from "./window.ipc";
