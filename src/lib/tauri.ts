import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (window.__TAURI_INTERNALS__) {
    return tauriInvoke(cmd, args);
  }
  console.warn(`[CRVI-GRC] Tauri invoke '${cmd}' (not in Tauri context)`);
  throw new Error("Application non lancée depuis Tauri. Utilisez le binaire compilé.");
}

export async function invokeSafe<T>(cmd: string, args?: Record<string, unknown>, fallback?: T): Promise<T | undefined> {
  try { return await invoke<T>(cmd, args); } catch { return fallback; }
}
