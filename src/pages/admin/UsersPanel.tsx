import { useState } from "react";
import toast from "react-hot-toast";
import type { RoleDetails, UserInput, UserSummary } from "../../types";
import { invoke } from "../../lib/tauri";
import { useAuth } from "../../lib/auth";
import { EMPTY_USER } from "./constants";

export function UsersPanel({
  users, roles, onRefresh, onPassword,
}: {
  users: UserSummary[];
  roles: RoleDetails[];
  onRefresh: () => Promise<void>;
  onPassword: (userId: number) => void;
}) {
  const { session, refreshSession } = useAuth();
  const [showUserForm, setShowUserForm] = useState(false);
  const [userForm, setUserForm] = useState<UserInput>(EMPTY_USER);

const assignableRoles = roles.filter((role) => role.code_role !== "PUBLIC");

  const saveUser = async () => {
    try {
      const savedUser = await invoke<UserSummary>("sauvegarder_user", { user: userForm });
      toast.success(userForm.id_user ? "Compte modifié" : "Compte créé");
      setUserForm(EMPTY_USER);
      if (session?.user_id === savedUser.id_user) {
        await refreshSession();
      }
      await onRefresh();
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
      await onRefresh();
    } catch (e) {
      toast.error(String(e));
    }
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
                    <button onClick={() => onPassword(user.id_user)} className="rounded-lg border px-2 py-1 text-xs hover:bg-muted cursor-pointer">Mot de passe</button>
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
  );
}
