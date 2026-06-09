import { useState, useEffect, useRef } from "react";
import type { EditLockStatus } from "../types";
import { invoke } from "../lib/tauri";

const RENEW_INTERVAL_MS = 4 * 60 * 1000; // 4 minutes

export function useEditLock(resourceType: string, resourceId: number | null | undefined, enabled: boolean) {
  const [lockStatus, setLockStatus] = useState<EditLockStatus | null>(null);
  const [lockLoading, setLockLoading] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const activeRef = useRef(false);

  // Cleanup: cancel timer and release lock
  useEffect(() => {
    if (!enabled || !resourceId) {
      setLockStatus(null);
      setLockLoading(false);
      return;
    }

    activeRef.current = true;

    const acquire = () => {
      if (!activeRef.current) return;
      setLockLoading(true);

      invoke<EditLockStatus>("acquire_edit_lock", { resourceType, resourceId })
        .then((status) => {
          if (activeRef.current) setLockStatus(status);
        })
        .catch(() => {
          if (activeRef.current) setLockStatus(null);
        })
        .finally(() => {
          if (activeRef.current) setLockLoading(false);
        });
    };

    // First acquisition
    acquire();

    // Periodic renewal
    timerRef.current = setInterval(() => {
      if (!activeRef.current) return;
      invoke<EditLockStatus>("acquire_edit_lock", { resourceType, resourceId })
        .then((status) => {
          if (!activeRef.current) return;
          if (!status.acquired) {
            // Lock was lost to another instance
            setLockStatus(status);
            // Cancel further renewals since we no longer own the lock
            if (timerRef.current) {
              clearInterval(timerRef.current);
              timerRef.current = null;
            }
          } else {
            setLockStatus(status);
          }
        })
        .catch(() => {
          if (activeRef.current) {
            setLockStatus({ acquired: false, resource_type: resourceType, resource_id: resourceId, holder_label: null, expires_at: null });
          }
        });
    }, RENEW_INTERVAL_MS);

    return () => {
      activeRef.current = false;

      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }

      void invoke("release_edit_lock", { resourceType, resourceId }).catch(() => {});
    };
  }, [enabled, resourceId, resourceType]);

  return {
    lockStatus,
    lockLoading,
    lockBlocked: !!lockStatus && !lockStatus.acquired,
  };
}
