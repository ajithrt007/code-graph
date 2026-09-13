import { useCallback, useEffect, useRef, useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";
import { version as appVersion } from "../../package.json";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "downloading"
  | "installed"
  | "error";

/** How long the silent startup check may take before giving up. */
const STARTUP_CHECK_TIMEOUT_MS = 20_000;

const message = (value: unknown): string => {
  if (value instanceof Error) return value.message;
  if (typeof value === "string") return value;
  if (value !== null && typeof value === "object") {
    const record = value as Record<string, unknown>;
    if (typeof record.message === "string" && record.message) {
      return record.message;
    }
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  return String(value);
};

export interface AvailableUpdate {
  version: string;
  body: string | undefined;
}

export function useUpdater(autoCheck: boolean) {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [update, setUpdate] = useState<AvailableUpdate | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const updateRef = useRef<import("@tauri-apps/plugin-updater").Update | null>(
    null,
  );
  const autoChecked = useRef(false);

  const checkForUpdates = useCallback(async (timeout = STARTUP_CHECK_TIMEOUT_MS) => {
    setStatus("checking");
    setError(null);
    setProgress(null);
    try {
      const found = await check({ timeout });
      if (!found) {
        updateRef.current = null;
        setUpdate(null);
        setStatus("up-to-date");
        return;
      }
      updateRef.current = found;
      setUpdate({ version: found.version, body: found.body });
      setStatus("available");
    } catch (err) {
      updateRef.current = null;
      setUpdate(null);
      setError(message(err));
      setStatus("error");
    }
  }, []);

  const downloadAndInstall = useCallback(async () => {
    const pending = updateRef.current;
    if (!pending) return;
    setStatus("downloading");
    setError(null);
    setProgress(0);
    let total: number | undefined;
    let downloaded = 0;
    try {
      await pending.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started" && event.data.contentLength) {
          total = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total) setProgress(Math.min(100, Math.round((downloaded / total) * 100)));
        } else if (event.event === "Finished") {
          setProgress(100);
        }
      });
      setStatus("installed");
    } catch (err) {
      setError(message(err));
      setStatus("error");
    }
  }, []);

  const restartNow = useCallback(async () => {
    try {
      await relaunch();
    } catch (err) {
      setError(message(err));
      setStatus("error");
    }
  }, []);

  // Silent startup check: surfaces availability, never installs.
  // Guarded for StrictMode double-mount; never blocks rendering.
  useEffect(() => {
    if (!autoCheck || autoChecked.current) return;
    autoChecked.current = true;
    void checkForUpdates();
  }, [autoCheck, checkForUpdates]);

  return {
    appVersion,
    status,
    update,
    progress,
    error,
    checkForUpdates,
    downloadAndInstall,
    restartNow,
  };
}
