import { useCallback, useEffect, useRef, useState } from "react";
import { diagnosticsIpc } from "../../services/ipc";

export type DiagnosticsFeedbackStatus = "idle" | "copied" | "copy-error" | "folder-error";

const COPIED_FEEDBACK_RESET_MS = 3000;

export function useDiagnostics(isActive = true) {
  const [copying, setCopying] = useState(false);
  const [openingFolder, setOpeningFolder] = useState(false);
  const [status, setStatus] = useState<DiagnosticsFeedbackStatus>("idle");
  const isMountedRef = useRef(true);
  const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearResetTimer = useCallback(() => {
    if (resetTimerRef.current !== null) {
      clearTimeout(resetTimerRef.current);
      resetTimerRef.current = null;
    }
  }, []);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      clearResetTimer();
    };
  }, [clearResetTimer]);

  useEffect(() => {
    if (!isActive) {
      clearResetTimer();
      setStatus("idle");
    }
  }, [isActive, clearResetTimer]);

  const copySummary = useCallback(async () => {
    if (copying) return;
    clearResetTimer();
    setCopying(true);
    setStatus("idle");
    try {
      const summary = await diagnosticsIpc.getDiagnosticSummary();
      const text = summary?.formattedText?.trim();
      if (!text || !navigator.clipboard?.writeText) {
        throw new Error("Clipboard unavailable");
      }
      await navigator.clipboard.writeText(text);
      if (!isMountedRef.current) return;
      setStatus("copied");
      resetTimerRef.current = setTimeout(() => {
        if (isMountedRef.current) {
          setStatus("idle");
        }
        resetTimerRef.current = null;
      }, COPIED_FEEDBACK_RESET_MS);
    } catch {
      if (isMountedRef.current) {
        setStatus("copy-error");
      }
    } finally {
      if (isMountedRef.current) {
        setCopying(false);
      }
    }
  }, [copying, clearResetTimer]);

  const openLogsFolder = useCallback(async () => {
    if (openingFolder) return;
    clearResetTimer();
    setOpeningFolder(true);
    setStatus((prev) => (prev === "folder-error" ? "idle" : prev));
    try {
      await diagnosticsIpc.openLogsFolder();
    } catch {
      if (isMountedRef.current) {
        setStatus("folder-error");
      }
    } finally {
      if (isMountedRef.current) {
        setOpeningFolder(false);
      }
    }
  }, [openingFolder, clearResetTimer]);

  return {
    copying,
    openingFolder,
    status,
    copySummary,
    openLogsFolder,
  };
}
