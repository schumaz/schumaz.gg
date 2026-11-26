import {
  SCHUMAZ_ASSET_PROTOCOL,
  assetsIpc,
  isTauriRuntime,
} from "../services/ipc/assets.ipc";

export { SCHUMAZ_ASSET_PROTOCOL, isTauriRuntime };

const ALLOWED_ASSET_PREFIXES = [
  "https://ddragon.leagueoflegends.com/",
  "https://raw.communitydragon.org/",
] as const;

const VALID_ASSET_EXTENSIONS = new Set(["png", "jpg", "jpeg", "webp", "svg", "gif"]);
const MAX_ASSET_URL_CACHE_SIZE = 2048;
const tauriAssetUrlCache = new Map<string, string>();

export function isAllowedAssetUrl(url: string): boolean {
  const trimmed = url.trim();
  return ALLOWED_ASSET_PREFIXES.some((prefix) => trimmed.startsWith(prefix));
}

export function encodeBase64Url(input: string): string {
  const bytes = new TextEncoder().encode(input);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
}

export function extractAssetExtension(url: string): string {
  const withoutQuery = url.split("?")[0] ?? url;
  const withoutHash = withoutQuery.split("#")[0] ?? withoutQuery;
  const lastDot = withoutHash.lastIndexOf(".");
  if (lastDot < 0) {
    return "bin";
  }
  const ext = withoutHash.slice(lastDot + 1).toLowerCase();
  return VALID_ASSET_EXTENSIONS.has(ext) ? ext : "bin";
}

export function buildAssetSegment(url: string): string {
  const trimmed = url.trim();
  const b64 = encodeBase64Url(trimmed);
  const ext = extractAssetExtension(trimmed);
  return `v1_${b64}.${ext}`;
}

/**
 * Resolves a remote Riot / CommunityDragon asset URL into the local
 * `schumaz-asset` URI scheme when running inside Tauri so it is cached on disk
 * (`%LOCALAPPDATA%\schumazgg\Cache\assets\`). Outside Tauri (jsdom/Vitest),
 * returns the remote URL verbatim.
 */
export function resolveCachedAssetUrl(url: string | null | undefined, forceTauri?: boolean): string {
  if (!url) {
    return "";
  }
  const trimmed = url.trim();
  if (!trimmed || !isAllowedAssetUrl(trimmed)) {
    return trimmed;
  }
  const useTauri = forceTauri ?? isTauriRuntime();
  if (!useTauri) {
    return trimmed;
  }
  if (forceTauri === undefined) {
    const cached = tauriAssetUrlCache.get(trimmed);
    if (cached !== undefined) {
      return cached;
    }
  }
  const resolved = assetsIpc.convertAssetFileSrc(buildAssetSegment(trimmed), SCHUMAZ_ASSET_PROTOCOL);
  if (forceTauri === undefined) {
    if (tauriAssetUrlCache.size >= MAX_ASSET_URL_CACHE_SIZE) {
      tauriAssetUrlCache.clear();
    }
    tauriAssetUrlCache.set(trimmed, resolved);
  }
  return resolved;
}
