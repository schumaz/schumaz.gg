import { useEffect, useState } from "react";
import { events } from "../types";

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

export function useWindowMinimized(): boolean {
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

    const handleBlur = () => {
      setIsWindowUnfocused(true);
    };

    const handleFocus = () => {
      setIsWindowUnfocused(false);
    };

    window.addEventListener("blur", handleBlur);
    window.addEventListener("focus", handleFocus);
    return () => {
      window.removeEventListener("blur", handleBlur);
      window.removeEventListener("focus", handleFocus);
    };
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    try {
      const listener = events?.windowMinimizedChanged?.listen;
      if (typeof listener === "function") {
        listener((event) => {
          if (active) {
            setIsWindowMinimized(Boolean(event.payload));
          }
        })
          .then((fn) => {
            if (active) {
              unlisten = fn;
            } else {
              fn();
            }
          })
          .catch(() => {});
      }
    } catch {
      // Non-Tauri environments (e.g. Vitest / browser) safely fall back to visibilitychange & blur/focus
    }

    return () => {
      active = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  return isDocumentHidden || isWindowUnfocused || isWindowMinimized;
}
