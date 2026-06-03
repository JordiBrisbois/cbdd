import { useState } from "react";
import toast from "react-hot-toast";
import { invoke } from "../../lib/tauri";
import { useAuth } from "../../lib/auth";

export function PasswordModal({ passwordUserId, onClose }: { passwordUserId: number | null; onClose: () => void }) {
  const { session, refreshSession } = useAuth();
  const [newPassword, setNewPassword] = useState("");

  if (!passwordUserId) return null;

  const savePassword = async () => {
    if (!newPassword.trim()) return;
    try {
      await invoke("changer_mot_de_passe_user", { payload: { userId: passwordUserId, newPassword } });
      toast.success("Mot de passe modifié");
      if (session?.user_id === passwordUserId) {
        await refreshSession();
      }
      setNewPassword("");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <div className="rounded-2xl border bg-card p-5 shadow-sm">
      <h2 className="text-lg font-semibold">Changer un mot de passe</h2>
      <p className="mt-2 text-sm text-muted-foreground">Le nouveau mot de passe sera appliqué immédiatement par l'administrateur.</p>
      <div className="mt-4 flex flex-wrap gap-3">
        <input value={newPassword} onChange={(e) => setNewPassword(e.target.value)} type="password" placeholder="Nouveau mot de passe"
          className="h-10 min-w-[260px] rounded-xl border bg-background px-3 text-sm" />
        <button onClick={() => void savePassword()} className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer">Mettre à jour</button>
        <button onClick={onClose} className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer">Annuler</button>
      </div>
    </div>
  );
}
