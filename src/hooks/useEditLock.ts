import { useEffect, useState } from "react";
import toast from "react-hot-toast";
import type { EditLockStatus } from "../types";
import { invoke } from "../lib/tauri";

const RENEW_INTERVAL_MS = 4 * 60 * 1000;

function lostLock(resourceType: string, resourceId: number): EditLockStatus {
  return {
    acquired: false,
    resource_type: resourceType,
    resource_id: resourceId,
    holder_label: null,
    expires_at: null,
  };
}

export function useEditLock(resourceType: string, resourceId: number | null | undefined, enabled: boolean) {
  const [lockStatus, setLockStatus] = useState<EditLockStatus | null>(null);
  const [lockLoading, setLockLoading] = useState(enabled && !!resourceId);

  useEffect(() => {
    if (!enabled || !resourceId) {
      setLockStatus(null);
      setLockLoading(false);
      return;
    }

    let active = true;
    let renewalTimer: ReturnType<typeof setInterval> | undefined;

    const stopRenewal = () => {
      if (renewalTimer) {
        clearInterval(renewalTimer);
        renewalTimer = undefined;
      }
    };

    const markLost = (status = lostLock(resourceType, resourceId), showError = false) => {
      if (!active) return;
      stopRenewal();
      setLockStatus(status);
      if (showError) {
        toast.error("Le verrou d'édition a été perdu. La fiche passe en lecture seule.");
      }
    };

    const renew = async () => {
      try {
        const status = await invoke<EditLockStatus>("acquire_edit_lock", { resourceType, resourceId });
        if (!active) return;
        if (!status.acquired) {
          markLost(status);
          return;
        }
        setLockStatus(status);
      } catch {
        markLost(undefined, true);
      }
    };

    const acquire = async () => {
      setLockLoading(true);
      try {
        const status = await invoke<EditLockStatus>("acquire_edit_lock", { resourceType, resourceId });
        if (!active) return;
        setLockStatus(status);
        if (status.acquired) {
          renewalTimer = setInterval(() => void renew(), RENEW_INTERVAL_MS);
        }
      } catch {
        markLost(undefined, true);
      } finally {
        if (active) setLockLoading(false);
      }
    };

    void acquire();

    return () => {
      active = false;
      stopRenewal();
      void invoke("release_edit_lock", { resourceType, resourceId }).catch(() => {});
    };
  }, [enabled, resourceId, resourceType]);

  const lockOwned = lockStatus?.acquired
    && lockStatus.resource_type === resourceType
    && lockStatus.resource_id === resourceId;

  return {
    lockStatus,
    lockLoading,
    lockBlocked: enabled && !!resourceId && (lockLoading || !lockOwned),
  };
}
