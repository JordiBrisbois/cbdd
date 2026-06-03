import { useState } from "react";
import toast from "react-hot-toast";
import type { Permission, RoleDetails, RoleInput } from "../../types";
import { invoke } from "../../lib/tauri";
import { EMPTY_ROLE } from "./constants";

export function RolesPanel({
  roles, permissions, onRefresh,
}: {
  roles: RoleDetails[];
  permissions: Permission[];
  onRefresh: () => Promise<void>;
}) {
  const [showRoleForm, setShowRoleForm] = useState(false);
  const [roleForm, setRoleForm] = useState<RoleInput>(EMPTY_ROLE);
  const selectedRoleDetails = roles.find((role) => role.id_role === roleForm.id_role) ?? null;
  const isEditingAdminRole = selectedRoleDetails?.code_role === "ADMIN";
  const isEditingSystemRole = !!selectedRoleDetails?.is_system;

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
      await onRefresh();
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
      await onRefresh();
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

  return (
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
  );
}
