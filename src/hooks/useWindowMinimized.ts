import { useEffect, useState } from "react";
import { events } from "../types";

function getDocumentHidden(): boolean {
  if (typeof document === "undefined") return false;
  return document.visibilityState === "hidden";
}

export function useWindowMinimized(): boolean {
  const [isDocumentHidden, setIsDocumentHidden] = useState<boolean>(getDocumentHidden);
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
      // Non-Tauri environments (e.g. Vitest / browser) safely fall back to visibilitychange
    }

    return () => {
      active = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  return isDocumentHidden || isWindowMinimized;
}
