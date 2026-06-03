import { useState, useEffect, useCallback } from "react";
import toast from "react-hot-toast";
import type {
  BackupInfo,
  Permission,
  RoleDetails,
  SecuritySettings,
  UserSummary,
} from "../../types";
import { invoke } from "../../lib/tauri";

export function useAdminData(canManageUsers: boolean, canManageRoles: boolean, canManageSettings: boolean, canManageBackups: boolean) {
  const [users, setUsers] = useState<UserSummary[]>([]);
  const [roles, setRoles] = useState<RoleDetails[]>([]);
  const [permissions, setPermissions] = useState<Permission[]>([]);
  const [settings, setSettings] = useState<SecuritySettings>({ anonymous_access_enabled: true });
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const tasks: Promise<void>[] = [];

      if (canManageUsers) {
        tasks.push(
          invoke<UserSummary[]>("lister_users").then(setUsers).catch((e) => {
            setUsers([]);
            throw e;
          }),
        );
      } else {
        setUsers([]);
      }

      if (canManageRoles) {
        tasks.push(
          invoke<RoleDetails[]>("lister_roles").then(setRoles).catch((e) => {
            setRoles([]);
            throw e;
          }),
        );
        tasks.push(
          invoke<Permission[]>("lister_permissions").then(setPermissions).catch((e) => {
            setPermissions([]);
            throw e;
          }),
        );
      } else {
        setRoles([]);
        setPermissions([]);
      }

      if (canManageSettings) {
        tasks.push(
          invoke<SecuritySettings>("get_security_settings").then(setSettings).catch((e) => {
            setSettings({ anonymous_access_enabled: true });
            throw e;
          }),
        );
      } else {
        setSettings({ anonymous_access_enabled: true });
      }

      if (canManageBackups) {
        tasks.push(
          invoke<BackupInfo[]>("list_local_backups").then(setBackups).catch((e) => {
            setBackups([]);
            throw e;
          }),
        );
      } else {
        setBackups([]);
      }

      await Promise.all(tasks);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [canManageBackups, canManageRoles, canManageSettings, canManageUsers]);

  useEffect(() => {
    void load();
  }, [load]);

  const refreshBackups = async () => {
    if (!canManageBackups) return;
    try {
      const items = await invoke<BackupInfo[]>("list_local_backups");
      setBackups(items);
    } catch (e) {
      toast.error(String(e));
    }
  };

  return {
    users, setUsers,
    roles, setRoles,
    permissions, setPermissions,
    settings, setSettings,
    backups, setBackups,
    loading, setLoading,
    refreshBackups,
    load,
  };
}
