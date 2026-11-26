import { useEffect, useState } from "react";
import { isInGamePhase } from "../features/champ-select/lcuBuildActions";
import { windowIpc, type UnlistenFn } from "../services/ipc";

export type WindowGraphicsMode = "focused" | "unfocused" | "zero-gpu";

export interface WindowGraphicsState {
  mode: WindowGraphicsMode;
  isMinimizedOrHidden: boolean;
  isWindowUnfocused: boolean;
  isInGame: boolean;
  isZeroGpu: boolean;
  animationsPaused: boolean;
  shellClassName: string;
}

function getDocumentHidden(): boolean {
  if (typeof document === "undefined") return false;
  return document.visibilityState === "hidden";
}

function getWindowUnfocused(): boolean {
  if (typeof document === "undefined") return false;
  if (typeof document.hasFocus === "function") {
    return !document.hasFocus();
  }
  return false;
}

export function resolveWindowGraphicsMode(input: {
  isDocumentHidden: boolean;
  isWindowMinimized: boolean;
  isWindowUnfocused: boolean;
  gameflowPhase?: string;
}): WindowGraphicsMode {
  if (
    input.isDocumentHidden ||
    input.isWindowMinimized ||
    isInGamePhase(input.gameflowPhase ?? "")
  ) {
    return "zero-gpu";
  }
  if (input.isWindowUnfocused) {
    return "unfocused";
  }
  return "focused";
}

export function useWindowGraphicsMode(gameflowPhase = ""): WindowGraphicsState {
  const [isDocumentHidden, setIsDocumentHidden] = useState<boolean>(getDocumentHidden);
  const [isWindowUnfocused, setIsWindowUnfocused] = useState<boolean>(getWindowUnfocused);
  const [isWindowMinimized, setIsWindowMinimized] = useState<boolean>(false);

  useEffect(() => {
    if (typeof document === "undefined") return;

    const handleVisibilityChange = () => {
      setIsDocumentHidden(getDocumentHidden());
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return;

    let active = true;
    let receivedFocusEvent = false;
    let unlistenFocus: UnlistenFn | undefined;

    const handleBlur = () => {
      receivedFocusEvent = true;
      setIsWindowUnfocused(true);
    };

    const handleFocus = () => {
      receivedFocusEvent = true;
      setIsWindowUnfocused(false);
    };

    window.addEventListener("blur", handleBlur);
    window.addEventListener("focus", handleFocus);

    windowIpc
      .isWindowFocused()
      .then((focused) => {
        if (active && !receivedFocusEvent && focused !== null) {
          setIsWindowUnfocused(!focused);
        }
      })
      .catch(() => {});

    windowIpc
      .onWindowFocusChanged((focused) => {
        receivedFocusEvent = true;
        if (active) {
          setIsWindowUnfocused(!focused);
        }
      })
      .then((fn) => {
        if (active) {
          unlistenFocus = fn;
        } else {
          fn();
        }
      })
      .catch(() => {});

    return () => {
      active = false;
      window.removeEventListener("blur", handleBlur);
      window.removeEventListener("focus", handleFocus);
      if (unlistenFocus) {
        unlistenFocus();
      }
    };
  }, []);

  useEffect(() => {
    let active = true;
    let receivedMinimizedEvent = false;
    let unlisten: UnlistenFn | undefined;

    windowIpc
      .isWindowMinimized()
      .then((minimized) => {
        if (active && !receivedMinimizedEvent && minimized !== null) {
          setIsWindowMinimized(Boolean(minimized));
        }
      })
      .catch(() => {});

    windowIpc
      .onWindowMinimizedChanged((minimized) => {
        receivedMinimizedEvent = true;
        if (active) {
          setIsWindowMinimized(Boolean(minimized));
        }
      })
      .then((fn) => {
        if (active) {
          unlisten = fn;
        } else {
          fn();
        }
      })
      .catch(() => {
        // Non-Tauri environments (e.g. Vitest / browser) safely fall back to visibilitychange & blur/focus
      });

    return () => {
      active = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const isMinimizedOrHidden = isDocumentHidden || isWindowMinimized;
  const isInGame = isInGamePhase(gameflowPhase);
  const mode = resolveWindowGraphicsMode({
    isDocumentHidden,
    isWindowMinimized,
    isWindowUnfocused,
    gameflowPhase,
  });

  return {
    mode,
    isMinimizedOrHidden,
    isWindowUnfocused,
    isInGame,
    isZeroGpu: mode === "zero-gpu",
    animationsPaused: mode !== "focused",
    shellClassName:
      mode === "zero-gpu" ? "app-minimized" : mode === "unfocused" ? "app-unfocused" : "",
  };
}

export function useWindowMinimized(gameflowPhase = ""): boolean {
  return useWindowGraphicsMode(gameflowPhase).isZeroGpu;
}
