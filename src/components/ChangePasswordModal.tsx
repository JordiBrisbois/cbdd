import { useState } from "react";
import toast from "react-hot-toast";
import { invoke, Modal } from "../lib/utils";

export function ChangePasswordModal({
  onClose,
  onChanged,
}: {
  onClose: () => void;
  onChanged?: () => Promise<void> | void;
}) {
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [saving, setSaving] = useState(false);

  const save = async () => {
    if (!currentPassword.trim() || !newPassword.trim()) {
      toast.error("Tous les champs sont requis");
      return;
    }
    if (newPassword.trim().length < 8) {
      toast.error("Le nouveau mot de passe doit contenir au moins 8 caractères");
      return;
    }
    if (newPassword !== confirmPassword) {
      toast.error("La confirmation ne correspond pas");
      return;
    }

    setSaving(true);
    try {
      await invoke("changer_mon_mot_de_passe", {
        payload: {
          current_password: currentPassword,
          new_password: newPassword,
        },
      });
      toast.success("Mot de passe mis à jour");
      await onChanged?.();
      onClose();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal open={true} onClose={onClose} title="Changer mon mot de passe">
      <div className="space-y-4">
        <p className="text-sm text-muted-foreground">
          Choisis un mot de passe personnel plus solide que le mot de passe d’installation.
        </p>
        <div>
          <label className="text-xs font-medium text-muted-foreground">Mot de passe actuel</label>
          <input
            type="password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            className="mt-1 h-10 w-full rounded-xl border bg-background px-3 text-sm"
          />
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground">Nouveau mot de passe</label>
          <input
            type="password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            className="mt-1 h-10 w-full rounded-xl border bg-background px-3 text-sm"
          />
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground">Confirmer le nouveau mot de passe</label>
          <input
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            className="mt-1 h-10 w-full rounded-xl border bg-background px-3 text-sm"
          />
        </div>
        <div className="flex justify-end gap-3">
          <button onClick={onClose} className="rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted cursor-pointer">
            Annuler
          </button>
          <button
            onClick={() => void save()}
            disabled={saving}
            className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60 cursor-pointer"
          >
            {saving ? "Enregistrement..." : "Mettre à jour"}
          </button>
        </div>
      </div>
    </Modal>
  );
}
