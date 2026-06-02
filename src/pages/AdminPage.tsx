import { useEffect, useState } from "react";
import toast from "react-hot-toast";
import type {
  BackupInfo,
  BackupRunResult,
  ExcelRebuildResult,
  ManualBackupRequest,
  Permission,
  RestoreBackupRequest,
  RoleDetails,
  RoleInput,
  SecuritySettings,
  UserInput,
  UserSummary,
} from "../types";
import { invoke, Modal } from "../lib/utils";
import { useAuth } from "../lib/auth";

const DEFAULT_UPDATE_ENDPOINT = "https://github.com/JordiBrisbois/cbdd/releases/latest/download/latest.json";

const EMPTY_USER: UserInput = {
  username: "",
  display_name: "",
  is_active: true,
  must_change_password: false,
  role_ids: [],
  password: "",
};

const EMPTY_ROLE: RoleInput = {
  nom_role: "",
  permission_codes: [],
};

export function AdminPage() {
  const { session, refreshSession, can } = useAuth();
  const [users, setUsers] = useState<UserSummary[]>([]);
  const [roles, setRoles] = useState<RoleDetails[]>([]);
  const [permissions, setPermissions] = useState<Permission[]>([]);
  const [settings, setSettings] = useState<SecuritySettings>({ anonymous_access_enabled: true });
  const [loading, setLoading] = useState(true);
  const [exportingWorkbook, setExportingWorkbook] = useState(false);
  const [runningLocalBackup, setRunningLocalBackup] = useState(false);
  const [runningPortableBackup, setRunningPortableBackup] = useState(false);
  const [restoringBackup, setRestoringBackup] = useState<string | null>(null);
  const [deletingBackup, setDeletingBackup] = useState<string | null>(null);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [portablePassphrase, setPortablePassphrase] = useState("");
  const [portablePassphraseConfirm, setPortablePassphraseConfirm] = useState("");
  const [restorePromptBackup, setRestorePromptBackup] = useState<BackupInfo | null>(null);
  const [restorePassphrase, setRestorePassphrase] = useState("");
  const [showBackupHelpModal, setShowBackupHelpModal] = useState(false);
  const [backupDirectory, setBackupDirectory] = useState("");
  const [openingBackupDirectory, setOpeningBackupDirectory] = useState(false);
  const [showPortableBackupForm, setShowPortableBackupForm] = useState(false);
  const [showUserForm, setShowUserForm] = useState(false);
  const [showRoleForm, setShowRoleForm] = useState(false);
  const [userForm, setUserForm] = useState<UserInput>(EMPTY_USER);
  const [roleForm, setRoleForm] = useState<RoleInput>(EMPTY_ROLE);
  const [passwordUserId, setPasswordUserId] = useState<number | null>(null);
  const [newPassword, setNewPassword] = useState("");
  const [customUpdateEndpoint, setCustomUpdateEndpoint] = useState(() =>
    typeof window === "undefined" ? "" : localStorage.getItem("crvi-update-endpoint") || ""
  );
  const selectedRoleDetails = roles.find((role) => role.id_role === roleForm.id_role) ?? null;
  const isEditingAdminRole = selectedRoleDetails?.code_role === "ADMIN";
  const isEditingSystemRole = !!selectedRoleDetails?.is_system;
  const assignableRoles = roles.filter((role) => role.code_role !== "PUBLIC");
  const canManageUsers = can("admin.users");
  const canManageRoles = can("admin.roles");
  const canManageSettings = can("admin.settings");
  const canExportWorkbook = can("admin.exports");
  const canManageBackups = can("admin.backups");

  const load = async () => {
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
  };

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => {
    void load();
  }, [canManageBackups, canManageRoles, canManageSettings, canManageUsers]);

  const saveSettings = async () => {
    try {
      const next = await invoke<SecuritySettings>("sauvegarder_security_settings", { settings });
      setSettings(next);
      const nextEndpoint = customUpdateEndpoint.trim();
      if (nextEndpoint) {
        localStorage.setItem("crvi-update-endpoint", nextEndpoint);
      } else {
        localStorage.removeItem("crvi-update-endpoint");
      }
      toast.success("Réglages de sécurité enregistrés");
    } catch (e) {
      toast.error(String(e));
    }
  };

  const exportWorkbook = async () => {
    setExportingWorkbook(true);
    try {
      const result = await invoke<ExcelRebuildResult | null>("exporter_classeur_excel_admin");
      if (!result) {
        toast("Export annulé");
        return;
      }
      toast.success(`${result.row_count} ligne(s) exportée(s) dans ${result.sheet_count} onglets`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExportingWorkbook(false);
    }
  };

  const refreshBackups = async () => {
    if (!canManageBackups) return;
    try {
      const items = await invoke<BackupInfo[]>("list_local_backups");
      setBackups(items);
    } catch (e) {
      toast.error(String(e));
    }
  };

  const openBackupHelp = async () => {
    setShowBackupHelpModal(true);
    if (backupDirectory) return;
    try {
      const path = await invoke<string>("get_backup_directory");
      setBackupDirectory(path);
    } catch (e) {
      toast.error(String(e));
    }
  };

  const openBackupDirectory = async () => {
    setOpeningBackupDirectory(true);
    try {
      const path = await invoke<string>("open_backup_directory");
      setBackupDirectory(path);
      toast.success("Dossier de sauvegarde ouvert.");
    } catch (e) {
      toast.error(String(e));
    } finally {
      setOpeningBackupDirectory(false);
    }
  };

  const createLocalBackup = async () => {
    setRunningLocalBackup(true);
    try {
      const request: ManualBackupRequest = { portable: false };
      const result = await invoke<BackupRunResult>("create_manual_backup", { request });
      toast.success(result.message);
      await refreshBackups();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setRunningLocalBackup(false);
    }
  };

  const createPortableBackup = async () => {
    if (portablePassphrase.length < 12) {
      toast.error("Le mot de passe portable doit contenir au moins 12 caractères.");
      return;
    }
    if (portablePassphrase !== portablePassphraseConfirm) {
      toast.error("Les deux mots de passe portables ne correspondent pas.");
      return;
    }

    setRunningPortableBackup(true);
    try {
      const request: ManualBackupRequest = {
        portable: true,
        passphrase: portablePassphrase,
      };
      const result = await invoke<BackupRunResult>("create_manual_backup", { request });
      toast.success(result.message);
      setPortablePassphrase("");
      setPortablePassphraseConfirm("");
      setShowPortableBackupForm(false);
      await refreshBackups();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setRunningPortableBackup(false);
    }
  };

  const startRestoreBackup = async (backup: BackupInfo) => {
    if (!confirm(`Restaurer le backup ${backup.name} ?\n\nAssure-toi que les autres postes n'écrivent plus dans la base.`)) {
      return;
    }

    if (backup.requires_passphrase) {
      setRestorePromptBackup(backup);
      setRestorePassphrase("");
      return;
    }

    await restoreBackup(backup, null);
  };

  const restoreBackup = async (backup: BackupInfo, passphrase: string | null) => {
    setRestoringBackup(backup.path);
    try {
      const request: RestoreBackupRequest = {
        backup_path: backup.path,
        passphrase,
      };
      const result = await invoke<BackupRunResult>("restore_local_backup", { request });
      toast.success(result.message);
      setRestorePromptBackup(null);
      setRestorePassphrase("");
      await refreshSession();
      await refreshBackups();
      await load();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setRestoringBackup(null);
    }
  };

  const deleteBackup = async (backup: BackupInfo) => {
    if (!confirm(`Supprimer définitivement le backup local ${backup.name} ?`)) {
      return;
    }

    setDeletingBackup(backup.path);
    try {
      await invoke("delete_local_backup", { backupPath: backup.path });
      toast.success("Backup supprimé");
      await refreshBackups();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setDeletingBackup(null);
    }
  };

  const saveUser = async () => {
    try {
      const savedUser = await invoke<UserSummary>("sauvegarder_user", { user: userForm });
      toast.success(userForm.id_user ? "Compte modifié" : "Compte créé");
      setUserForm(EMPTY_USER);
      if (session?.user_id === savedUser.id_user) {
        await refreshSession();
      }
      await load();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const editUser = (user: UserSummary) => {
    setShowUserForm(true);
    setUserForm({
      id_user: user.id_user,
      username: user.username,
      display_name: user.display_name,
      is_active: user.is_active,
      must_change_password: user.must_change_password,
      role_ids: user.role_ids,
      password: "",
    });
  };

  const deleteUser = async (userId: number) => {
    if (!confirm("Supprimer ce compte ?")) return;
    try {
      await invoke("supprimer_user", { userId });
      toast.success("Compte supprimé");
      if (userForm.id_user === userId) setUserForm(EMPTY_USER);
      if (session?.user_id === userId) {
        await refreshSession();
      }
      await load();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const saveRole = async () => {
    if (isEditingAdminRole) {
      toast.error("Le rôle Administrateur a déjà tous les accès par défaut");
      return;
    }
    try {
      await invoke<RoleDetails>("sauvegarder_role", { role: roleForm });
      toast.success(roleForm.id_role ? "Rôle modifié" : "Rôle créé");
      setRoleForm(EMPTY_ROLE);
      setShowRoleForm(false);
      await load();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const editRole = (role: RoleDetails) => {
    setShowRoleForm(true);
    setRoleForm({
      id_role: role.id_role,
      nom_role: role.nom_role,
      permission_codes: role.permission_codes,
    });
  };

  const deleteRole = async (roleId: number) => {
    if (!confirm("Supprimer ce rôle ?")) return;
    try {
      await invoke("supprimer_role", { roleId });
      toast.success("Rôle supprimé");
      if (roleForm.id_role === roleId) setRoleForm(EMPTY_ROLE);
      await load();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const savePassword = async () => {
    if (!passwordUserId || !newPassword.trim()) return;
    try {
      await invoke("changer_mot_de_passe_user", { payload: { userId: passwordUserId, newPassword } });
      toast.success("Mot de passe modifié");
      if (session?.user_id === passwordUserId) {
        await refreshSession();
      }
      setPasswordUserId(null);
      setNewPassword("");
    } catch (e) {
      toast.error(String(e));
    }
  };

  const toggleRolePermission = (permissionCode: string) => {
    setRoleForm((prev) => ({
      ...prev,
      permission_codes: prev.permission_codes.includes(permissionCode)
        ? prev.permission_codes.filter((code) => code !== permissionCode)
        : [...prev.permission_codes, permissionCode],
    }));
  };

  const toggleUserRole = (roleId: number) => {
    setUserForm((prev) => ({
      ...prev,
      role_ids: prev.role_ids.includes(roleId)
        ? prev.role_ids.filter((value) => value !== roleId)
        : [...prev.role_ids, roleId],
    }));
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Administration</h1>
          <p className="text-sm text-muted-foreground">Comptes, rôles, mots de passe et accès public.</p>
        </div>
        {loading && <span className="text-sm text-muted-foreground">Chargement...</span>}
      </div>

      {canManageSettings && (
        <div className="rounded-2xl border bg-card p-5 shadow-sm">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <h2 className="text-lg font-semibold">Accès public</h2>
              <p className="text-sm text-muted-foreground">Si activé, l'application reste accessible sans login avec les droits du rôle Public.</p>
            </div>
            <label className="flex items-center gap-3 rounded-xl border bg-background px-4 py-3 text-sm">
              <input
                type="checkbox"
                checked={settings.anonymous_access_enabled}
                onChange={(e) => setSettings({ anonymous_access_enabled: e.target.checked })}
                className="rounded border-gray-300"
              />
              Autoriser l'ouverture sans connexion
            </label>
          </div>
          <div className="mt-4 grid gap-2">
            <label className="text-xs font-medium text-muted-foreground">URL de mise à jour</label>
            <input
              value={customUpdateEndpoint}
              onChange={(e) => setCustomUpdateEndpoint(e.target.value)}
              placeholder={DEFAULT_UPDATE_ENDPOINT}
              className="h-10 rounded-xl border bg-background px-3 text-sm"
            />
            <p className="text-xs text-muted-foreground">
              Laisse vide pour utiliser l’URL de release par défaut. Ce réglage est local à ce poste.
            </p>
          </div>
          <div className="mt-4">
            <button onClick={() => void saveSettings()} className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer">
              Enregistrer la sécurité
            </button>
          </div>
        </div>
      )}

      {canExportWorkbook && (
        <div className="rounded-2xl border bg-card p-5 shadow-sm">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <h2 className="text-lg font-semibold">Export Excel historique</h2>
              <p className="text-sm text-muted-foreground">
                Reconstruit un classeur `.xlsx` à partir de SQLite avec une sheet par catégorie, dans une logique proche de l’ancien fichier.
              </p>
            </div>
            <button
              onClick={() => void exportWorkbook()}
              disabled={exportingWorkbook}
              className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
            >
              {exportingWorkbook ? "Export en cours..." : "Reconstruire le fichier Excel"}
            </button>
          </div>
          <p className="mt-3 text-xs text-muted-foreground">
            Les onglets suivent les catégories métier historiques (`AG`, `CA`, `CPAS`, `CRI`, `ILI`, etc.) et l’export se base d’abord sur la catégorie d’affiliation.
          </p>
        </div>
      )}

      {canManageBackups && (
        <div className="rounded-2xl border bg-card p-5 shadow-sm">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <h2 className="text-lg font-semibold">Sauvegardes SQLite</h2>
              <p className="text-sm text-muted-foreground">
                Les sauvegardes automatiques restent locales au poste, avec chiffrement Windows par défaut.
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <button
                onClick={() => void createLocalBackup()}
                disabled={runningLocalBackup || runningPortableBackup}
                className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
              >
                {runningLocalBackup ? "Backup local..." : "Créer un backup local sécurisé"}
              </button>
              <button
                onClick={() => setShowPortableBackupForm((value) => !value)}
                disabled={runningPortableBackup || runningLocalBackup}
                className="rounded-xl border px-4 py-2 text-sm font-medium hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
              >
                {showPortableBackupForm ? "Masquer le backup portable" : "Backup portable chiffré"}
              </button>
              <button
                onClick={() => void openBackupHelp()}
                className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer"
              >
                Sécurité / récupération
              </button>
            </div>
          </div>
          {showPortableBackupForm && (
            <div className="mt-4 rounded-xl border bg-background p-4">
              <div className="flex flex-wrap items-start justify-between gap-4">
                <div>
                  <p className="text-sm font-medium">Créer un backup portable chiffré</p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    À utiliser si tu veux sortir une copie du poste ou préparer un plan de reprise.
                  </p>
                </div>
                <button
                  onClick={() => void createPortableBackup()}
                  disabled={runningPortableBackup || runningLocalBackup}
                  className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {runningPortableBackup ? "Création..." : "Créer le backup portable"}
                </button>
              </div>
              <div className="mt-4 grid gap-3 md:grid-cols-2">
                <input
                  value={portablePassphrase}
                  onChange={(e) => setPortablePassphrase(e.target.value)}
                  type="password"
                  placeholder="Mot de passe de secours (12 caractères min.)"
                  className="h-10 rounded-xl border bg-card px-3 text-sm"
                />
                <input
                  value={portablePassphraseConfirm}
                  onChange={(e) => setPortablePassphraseConfirm(e.target.value)}
                  type="password"
                  placeholder="Confirmer le mot de passe"
                  className="h-10 rounded-xl border bg-card px-3 text-sm"
                />
              </div>
            </div>
          )}

          <div className="mt-4 overflow-x-auto rounded-xl border">
            <table className="w-full text-sm">
              <thead className="bg-muted/40 text-left">
                <tr>
                  <th className="px-3 py-2 font-medium">Nom</th>
                  <th className="px-3 py-2 font-medium">Créé le</th>
                  <th className="px-3 py-2 font-medium">Type</th>
                  <th className="px-3 py-2 font-medium">Protection</th>
                  <th className="px-3 py-2 font-medium">Taille</th>
                  <th className="px-3 py-2 font-medium">Actions</th>
                </tr>
              </thead>
              <tbody>
                {backups.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="px-3 py-4 text-sm text-muted-foreground">Aucun backup local disponible sur ce poste.</td>
                  </tr>
                ) : backups.map((backup) => (
                  <tr key={backup.path} className="border-t">
                    <td className="px-3 py-2 font-medium" title={backup.path}>
                      <div className="max-w-[340px] truncate">{backup.name}</div>
                    </td>
                    <td className="px-3 py-2 text-muted-foreground">{backup.created_at || "—"}</td>
                    <td className="px-3 py-2 text-muted-foreground">{backup.automatic ? "Auto" : "Manuel"}</td>
                    <td className="px-3 py-2 text-muted-foreground">
                      <div>{backup.encryption_label}</div>
                    </td>
                    <td className="px-3 py-2 text-muted-foreground">{(backup.size_bytes / 1024 / 1024).toFixed(2)} MB</td>
                    <td className="px-3 py-2">
                      <div className="flex flex-wrap gap-2">
                        <button
                          onClick={() => void startRestoreBackup(backup)}
                          disabled={restoringBackup === backup.path || deletingBackup === backup.path}
                          className="rounded-lg border px-2 py-1 text-xs hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          {restoringBackup === backup.path ? "Restauration..." : "Restaurer"}
                        </button>
                        <button
                          onClick={() => void deleteBackup(backup)}
                          disabled={deletingBackup === backup.path || restoringBackup === backup.path}
                          className="rounded-lg border px-2 py-1 text-xs text-red-700 hover:bg-red-50 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          {deletingBackup === backup.path ? "Suppression..." : "Supprimer"}
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <div className="grid gap-6 xl:grid-cols-[1.15fr_0.85fr]">
        {canManageUsers && (
        <section className="rounded-2xl border bg-card p-5 shadow-sm">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold">Comptes</h2>
            <button
              onClick={() => {
                setUserForm(EMPTY_USER);
                setShowUserForm((value) => !value || !!userForm.id_user);
              }}
              className="rounded-lg border bg-background px-3 py-1.5 text-sm hover:bg-muted cursor-pointer"
            >
              {showUserForm ? "Masquer le formulaire" : "Nouveau compte"}
            </button>
          </div>
          <div className="overflow-x-auto rounded-xl border">
            <table className="w-full text-sm">
              <thead className="bg-muted/40 text-left">
                <tr>
                  <th className="px-3 py-2 font-medium">Compte</th>
                  <th className="px-3 py-2 font-medium">Rôles</th>
                  <th className="px-3 py-2 font-medium">Statut</th>
                  <th className="px-3 py-2 font-medium">Actions</th>
                </tr>
              </thead>
              <tbody>
                {users.map((user) => (
                  <tr key={user.id_user} className="border-t">
                    <td className="px-3 py-2">
                      <div className="font-medium">{user.display_name || user.username}</div>
                      <div className="text-xs text-muted-foreground">{user.username}</div>
                    </td>
                    <td className="px-3 py-2 text-xs text-muted-foreground">
                      {user.role_codes.length === 0 ? "—" : user.role_codes.join(", ")}
                    </td>
                    <td className="px-3 py-2 text-xs">
                      <span className={`rounded-full px-2 py-1 ${user.is_active ? "bg-emerald-50 text-emerald-700" : "bg-slate-100 text-slate-600"}`}>
                        {user.is_active ? "Actif" : "Désactivé"}
                      </span>
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex flex-wrap gap-2">
                        <button onClick={() => editUser(user)} className="rounded-lg border px-2 py-1 text-xs hover:bg-muted cursor-pointer">Éditer</button>
                        <button onClick={() => setPasswordUserId(user.id_user)} className="rounded-lg border px-2 py-1 text-xs hover:bg-muted cursor-pointer">Mot de passe</button>
                        <button onClick={() => void deleteUser(user.id_user)} className="rounded-lg border border-red-200 px-2 py-1 text-xs text-red-700 hover:bg-red-50 cursor-pointer">Supprimer</button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {showUserForm && (
          <div className="mt-5 rounded-2xl border bg-background p-4">
            <h3 className="text-sm font-semibold">{userForm.id_user ? "Modifier le compte" : "Créer un compte"}</h3>
            <div className="mt-4 grid gap-4 md:grid-cols-2">
              <input value={userForm.username} onChange={(e) => setUserForm({ ...userForm, username: e.target.value })}
                placeholder="Email ou identifiant de connexion"
                className="h-10 rounded-xl border bg-card px-3 text-sm" />
              <input value={userForm.display_name ?? ""} onChange={(e) => setUserForm({ ...userForm, display_name: e.target.value })}
                placeholder="Nom affiché"
                className="h-10 rounded-xl border bg-card px-3 text-sm" />
              {!userForm.id_user && (
                <input value={userForm.password ?? ""} onChange={(e) => setUserForm({ ...userForm, password: e.target.value })}
                  placeholder="Mot de passe initial"
                  type="password"
                  className="h-10 rounded-xl border bg-card px-3 text-sm md:col-span-2" />
              )}
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={userForm.is_active} onChange={(e) => setUserForm({ ...userForm, is_active: e.target.checked })} />
                Compte actif
              </label>
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={userForm.must_change_password} onChange={(e) => setUserForm({ ...userForm, must_change_password: e.target.checked })} />
                Demander un changement de mot de passe
              </label>
            </div>
            <div className="mt-4">
              <p className="mb-2 text-xs font-medium text-muted-foreground">Rôles</p>
              <div className="mb-3 rounded-xl border bg-card px-3 py-2 text-xs text-muted-foreground">
                Le rôle <span className="font-medium text-foreground">Administrateur</span> donne l'accès complet à l'application.
                Le rôle <span className="font-medium text-foreground">Public</span> est réservé à l'accès anonyme et ne doit pas être attribué à un compte utilisateur.
              </div>
              <div className="flex flex-wrap gap-2">
                {assignableRoles.map((role) => (
                  <label key={role.id_role} className="flex items-center gap-2 rounded-xl border px-3 py-2 text-sm">
                    <input type="checkbox" checked={userForm.role_ids.includes(role.id_role)} onChange={() => toggleUserRole(role.id_role)} />
                    {role.nom_role}
                  </label>
                ))}
              </div>
            </div>
            <div className="mt-4 flex gap-2">
              <button onClick={() => void saveUser()} className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer">
                {userForm.id_user ? "Enregistrer le compte" : "Créer le compte"}
              </button>
              <button onClick={() => { setUserForm(EMPTY_USER); setShowUserForm(false); }} className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer">
                Réinitialiser
              </button>
            </div>
          </div>
          )}
        </section>
        )}

        {canManageRoles && (
        <section className="rounded-2xl border bg-card p-5 shadow-sm">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold">Rôles</h2>
            <button
              onClick={() => {
                setRoleForm(EMPTY_ROLE);
                setShowRoleForm((value) => !value || !!roleForm.id_role);
              }}
              className="rounded-lg border bg-background px-3 py-1.5 text-sm hover:bg-muted cursor-pointer"
            >
              {showRoleForm ? "Masquer le formulaire" : "Nouveau rôle"}
            </button>
          </div>
          <div className="space-y-3">
            {roles.map((role) => (
              <div key={role.id_role} className="rounded-xl border bg-background p-3">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className="font-medium">{role.nom_role}</p>
                    <p className="text-xs text-muted-foreground">{role.code_role}</p>
                    <p className="mt-2 text-xs text-muted-foreground">
                      {role.code_role === "ADMIN"
                        ? "Accès complet"
                        : `${role.permission_codes.length} permission(s)`}
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <button onClick={() => editRole(role)} className="rounded-lg border px-2 py-1 text-xs hover:bg-muted cursor-pointer">Éditer</button>
                    {!role.is_system && (
                      <button onClick={() => void deleteRole(role.id_role)} className="rounded-lg border border-red-200 px-2 py-1 text-xs text-red-700 hover:bg-red-50 cursor-pointer">Supprimer</button>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>

          {showRoleForm && (
          <div className="mt-5 rounded-2xl border bg-background p-4">
            <h3 className="text-sm font-semibold">{roleForm.id_role ? "Modifier le rôle" : "Créer un rôle"}</h3>
            <input value={roleForm.nom_role} onChange={(e) => setRoleForm({ ...roleForm, nom_role: e.target.value })}
              placeholder="Nom du rôle"
              disabled={isEditingSystemRole}
              className="mt-4 h-10 w-full rounded-xl border bg-card px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60" />
            {isEditingAdminRole && (
              <p className="mt-2 text-xs text-muted-foreground">
                Le rôle Administrateur garde automatiquement tous les accès. Il n'est pas nécessaire de lui attribuer des permissions.
              </p>
            )}
            {selectedRoleDetails?.code_role === "PUBLIC" && (
              <p className="mt-2 text-xs text-muted-foreground">
                Le rôle Public définit ce qu'un utilisateur non connecté peut consulter quand l'accès anonyme est activé.
              </p>
            )}
            <div className="mt-4 max-h-72 space-y-2 overflow-y-auto rounded-xl border bg-card p-3">
              {permissions.map((permission) => (
                <label key={permission.code} className="flex items-start gap-3 rounded-lg px-2 py-2 hover:bg-muted/40">
                  <input
                    type="checkbox"
                    checked={roleForm.permission_codes.includes(permission.code)}
                    onChange={() => toggleRolePermission(permission.code)}
                    disabled={isEditingAdminRole}
                    className="mt-1"
                  />
                  <span>
                    <span className="block text-sm font-medium">{permission.code}</span>
                    <span className="block text-xs text-muted-foreground">{permission.description}</span>
                  </span>
                </label>
              ))}
            </div>
            <div className="mt-4 flex gap-2">
              <button onClick={() => void saveRole()} disabled={isEditingAdminRole}
                className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
                {isEditingAdminRole ? "Rôle verrouillé" : roleForm.id_role ? "Enregistrer le rôle" : "Créer le rôle"}
              </button>
              <button onClick={() => { setRoleForm(EMPTY_ROLE); setShowRoleForm(false); }} className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer">
                Réinitialiser
              </button>
            </div>
          </div>
          )}
        </section>
        )}
      </div>

      {canManageUsers && passwordUserId && (
        <div className="rounded-2xl border bg-card p-5 shadow-sm">
          <h2 className="text-lg font-semibold">Changer un mot de passe</h2>
          <p className="mt-2 text-sm text-muted-foreground">Le nouveau mot de passe sera appliqué immédiatement par l'administrateur.</p>
          <div className="mt-4 flex flex-wrap gap-3">
            <input value={newPassword} onChange={(e) => setNewPassword(e.target.value)} type="password" placeholder="Nouveau mot de passe"
              className="h-10 min-w-[260px] rounded-xl border bg-background px-3 text-sm" />
            <button onClick={() => void savePassword()} className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer">
              Mettre à jour
            </button>
            <button onClick={() => { setPasswordUserId(null); setNewPassword(""); }} className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer">
              Annuler
            </button>
          </div>
        </div>
      )}

      <Modal
        open={showBackupHelpModal}
        onClose={() => {
          if (openingBackupDirectory) return;
          setShowBackupHelpModal(false);
        }}
        title="Sécurité et récupération"
      >
        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Les backups locaux automatiques et manuels sont protégés par Windows sur ce poste. Ils sont pensés pour la sécurité au quotidien et une restauration simple depuis l'application.
          </p>
          <p className="text-sm text-muted-foreground">
            Les backups portables sont chiffrés avec le mot de passe saisi lors de leur création. Ils servent surtout à sortir une copie du poste ou à préparer une reprise si l'application ne démarre plus.
          </p>
          <div className="rounded-xl border bg-background p-4">
            <p className="text-sm font-medium">Récupération hors application</p>
            <p className="mt-1 text-sm text-muted-foreground">
              Le dossier de sauvegarde contient aussi `README_RECOVERY.txt` et `decrypt-crvi-backup.ps1` pour déchiffrer un backup portable en dehors de l'application.
            </p>
            {backupDirectory && (
              <p className="mt-3 break-all rounded-lg bg-muted/40 px-3 py-2 font-mono text-xs text-muted-foreground">
                {backupDirectory}
              </p>
            )}
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={() => void openBackupDirectory()}
              disabled={openingBackupDirectory}
              className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
            >
              {openingBackupDirectory ? "Ouverture..." : "Ouvrir le dossier des backups"}
            </button>
            <button
              onClick={() => setShowBackupHelpModal(false)}
              disabled={openingBackupDirectory}
              className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
            >
              Fermer
            </button>
          </div>
        </div>
      </Modal>

      <Modal
        open={!!restorePromptBackup}
        onClose={() => {
          if (restoringBackup) return;
          setRestorePromptBackup(null);
          setRestorePassphrase("");
        }}
        title="Restaurer un backup portable"
      >
        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Ce backup a été chiffré avec un mot de passe portable. Saisis la passphrase utilisée lors de sa création pour lancer la restauration.
          </p>
          <input
            value={restorePassphrase}
            onChange={(e) => setRestorePassphrase(e.target.value)}
            type="password"
            placeholder="Mot de passe du backup portable"
            className="h-10 w-full rounded-xl border bg-background px-3 text-sm"
          />
          <div className="flex gap-2">
            <button
              onClick={() => restorePromptBackup && void restoreBackup(restorePromptBackup, restorePassphrase)}
              disabled={!restorePassphrase || !!restoringBackup}
              className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
            >
              {restoringBackup ? "Restauration..." : "Restaurer"}
            </button>
            <button
              onClick={() => {
                setRestorePromptBackup(null);
                setRestorePassphrase("");
              }}
              disabled={!!restoringBackup}
              className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
            >
              Annuler
            </button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
