import { useState } from "react";
import { useAuth } from "../../lib/auth";
import { useAdminData } from "./useAdminData";
import { SecurityPanel } from "./SecurityPanel";
import { ExportPanel } from "./ExportPanel";
import { BackupsPanel } from "./BackupsPanel";
import { UsersPanel } from "./UsersPanel";
import { RolesPanel } from "./RolesPanel";
import { PasswordModal } from "./AdminModals";

export function AdminPage() {
  const { can } = useAuth();
  const [passwordUserId, setPasswordUserId] = useState<number | null>(null);
  const canManageUsers = can("admin.users");
  const canManageRoles = can("admin.roles");
  const canManageSettings = can("admin.settings");
  const canExportWorkbook = can("admin.exports");
  const canManageBackups = can("admin.backups");

  const {
    users, roles, permissions, settings, setSettings, backups,
    loading, refreshBackups, load,
  } = useAdminData(canManageUsers, canManageRoles, canManageSettings, canManageBackups);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Administration</h1>
          <p className="text-sm text-muted-foreground">Comptes, rôles, mots de passe et accès public.</p>
        </div>
        {loading && <span className="text-sm text-muted-foreground">Chargement...</span>}
      </div>

      {canManageSettings && <SecurityPanel settings={settings} setSettings={setSettings} />}

      {canExportWorkbook && <ExportPanel />}

      {canManageBackups && (
        <BackupsPanel
          backups={backups}
          onRefreshBackups={refreshBackups}
          onLoad={load}
        />
      )}

      <div className="grid gap-6 xl:grid-cols-[1.15fr_0.85fr]">
        {canManageUsers && (
          <UsersPanel users={users} roles={roles} onRefresh={load} onPassword={setPasswordUserId} />
        )}

        {canManageRoles && (
          <RolesPanel roles={roles} permissions={permissions} onRefresh={load} />
        )}
      </div>

      {canManageUsers && <PasswordModal passwordUserId={passwordUserId} onClose={() => setPasswordUserId(null)} />}
    </div>
  );
}
