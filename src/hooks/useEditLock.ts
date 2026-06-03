import { useState, useEffect } from "react";
import type { EditLockStatus } from "../types";
import { invoke } from "../lib/tauri";

export function useEditLock(resourceType: string, resourceId: number | null | undefined, enabled: boolean) {
  const [lockStatus, setLockStatus] = useState<EditLockStatus | null>(null);
  const [lockLoading, setLockLoading] = useState(false);

  useEffect(() => {
    if (!enabled || !resourceId) {
      setLockStatus(null);
      setLockLoading(false);
      return;
    }

    let active = true;
    setLockLoading(true);

    void invoke<EditLockStatus>("acquire_edit_lock", { resourceType, resourceId })
      .then((status) => {
        if (active) setLockStatus(status);
      })
      .catch(() => {
        if (active) setLockStatus(null);
      })
      .finally(() => {
        if (active) setLockLoading(false);
      });

    return () => {
      active = false;
      void invoke("release_edit_lock", { resourceType, resourceId }).catch(() => {});
    };
  }, [enabled, resourceId, resourceType]);

  return {
    lockStatus,
    lockLoading,
    lockBlocked: !!lockStatus && !lockStatus.acquired,
  };
}
