import { useState } from "react";
import toast from "react-hot-toast";
import type { BackupInfo, BackupRunResult, ManualBackupRequest, RestoreBackupRequest } from "../../types";
import { invoke } from "../../lib/tauri";
import { useAuth } from "../../lib/auth";
import { Modal } from "../../components/Modal";

export function BackupsPanel({
  backups, onRefreshBackups, onLoad,
}: {
  backups: BackupInfo[];
  onRefreshBackups: () => Promise<void>;
  onLoad: () => Promise<void>;
}) {
  const { refreshSession } = useAuth();
  const [runningLocalBackup, setRunningLocalBackup] = useState(false);
  const [runningPortableBackup, setRunningPortableBackup] = useState(false);
  const [restoringBackup, setRestoringBackup] = useState<string | null>(null);
  const [deletingBackup, setDeletingBackup] = useState<string | null>(null);
  const [portablePassphrase, setPortablePassphrase] = useState("");
  const [portablePassphraseConfirm, setPortablePassphraseConfirm] = useState("");
  const [restorePromptBackup, setRestorePromptBackup] = useState<BackupInfo | null>(null);
  const [restorePassphrase, setRestorePassphrase] = useState("");
  const [showBackupHelpModal, setShowBackupHelpModal] = useState(false);
  const [backupDirectory, setBackupDirectory] = useState("");
  const [openingBackupDirectory, setOpeningBackupDirectory] = useState(false);
  const [showPortableBackupForm, setShowPortableBackupForm] = useState(false);

  const createLocalBackup = async () => {
    setRunningLocalBackup(true);
    try {
      const request: ManualBackupRequest = { portable: false };
      const result = await invoke<BackupRunResult>("create_manual_backup", { request });
      toast.success(result.message);
      await onRefreshBackups();
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
      await onRefreshBackups();
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
      await onRefreshBackups();
      await onLoad();
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
      await onRefreshBackups();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setDeletingBackup(null);
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

  const openBackupDirectoryBtn = async () => {
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

  return (
    <>
      <div className="rounded-2xl border bg-card p-5 shadow-sm">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold">Sauvegardes SQLite</h2>
            <p className="text-sm text-muted-foreground">Les sauvegardes automatiques restent locales au poste, avec chiffrement Windows par défaut.</p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button onClick={() => void createLocalBackup()} disabled={runningLocalBackup || runningPortableBackup}
              className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
              {runningLocalBackup ? "Backup local..." : "Créer un backup local sécurisé"}
            </button>
            <button onClick={() => setShowPortableBackupForm((value) => !value)} disabled={runningPortableBackup || runningLocalBackup}
              className="rounded-xl border px-4 py-2 text-sm font-medium hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
              {showPortableBackupForm ? "Masquer le backup portable" : "Backup portable chiffré"}
            </button>
            <button onClick={() => void openBackupHelp()} className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer">
              Sécurité / récupération
            </button>
          </div>
        </div>

        {showPortableBackupForm && (
          <div className="mt-4 rounded-xl border bg-background p-4">
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div>
                <p className="text-sm font-medium">Créer un backup portable chiffré</p>
                <p className="mt-1 text-xs text-muted-foreground">À utiliser si tu veux sortir une copie du poste ou préparer un plan de reprise.</p>
              </div>
              <button onClick={() => void createPortableBackup()} disabled={runningPortableBackup || runningLocalBackup}
                className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
                {runningPortableBackup ? "Création..." : "Créer le backup portable"}
              </button>
            </div>
            <div className="mt-4 grid gap-3 md:grid-cols-2">
              <input value={portablePassphrase} onChange={(e) => setPortablePassphrase(e.target.value)} type="password"
                placeholder="Mot de passe de secours (12 caractères min.)" className="h-10 rounded-xl border bg-card px-3 text-sm" />
              <input value={portablePassphraseConfirm} onChange={(e) => setPortablePassphraseConfirm(e.target.value)} type="password"
                placeholder="Confirmer le mot de passe" className="h-10 rounded-xl border bg-card px-3 text-sm" />
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
                <tr><td colSpan={6} className="px-3 py-4 text-sm text-muted-foreground">Aucun backup local disponible sur ce poste.</td></tr>
              ) : backups.map((backup) => (
                <tr key={backup.path} className="border-t">
                  <td className="px-3 py-2 font-medium" title={backup.path}>
                    <div className="max-w-[340px] truncate">{backup.name}</div>
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">{backup.created_at || "—"}</td>
                  <td className="px-3 py-2 text-muted-foreground">{backup.automatic ? "Auto" : "Manuel"}</td>
                  <td className="px-3 py-2 text-muted-foreground"><div>{backup.encryption_label}</div></td>
                  <td className="px-3 py-2 text-muted-foreground">{(backup.size_bytes / 1024 / 1024).toFixed(2)} MB</td>
                  <td className="px-3 py-2">
                    <div className="flex flex-wrap gap-2">
                      <button onClick={() => void startRestoreBackup(backup)}
                        disabled={restoringBackup === backup.path || deletingBackup === backup.path}
                        className="rounded-lg border px-2 py-1 text-xs hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
                        {restoringBackup === backup.path ? "Restauration..." : "Restaurer"}
                      </button>
                      <button onClick={() => void deleteBackup(backup)}
                        disabled={deletingBackup === backup.path || restoringBackup === backup.path}
                        className="rounded-lg border px-2 py-1 text-xs text-red-700 hover:bg-red-50 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
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

      <Modal open={showBackupHelpModal} onClose={() => { if (openingBackupDirectory) return; setShowBackupHelpModal(false); }} title="Sécurité et récupération">
        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">Les backups locaux automatiques et manuels sont protégés par Windows sur ce poste.</p>
          <p className="text-sm text-muted-foreground">Les backups portables sont chiffrés avec le mot de passe saisi lors de leur création.</p>
          <div className="rounded-xl border bg-background p-4">
            <p className="text-sm font-medium">Récupération hors application</p>
            <p className="mt-1 text-sm text-muted-foreground">Le dossier de sauvegarde contient README_RECOVERY.txt et decrypt-crvi-backup.ps1.</p>
            {backupDirectory && (
              <p className="mt-3 break-all rounded-lg bg-muted/40 px-3 py-2 font-mono text-xs text-muted-foreground">{backupDirectory}</p>
            )}
          </div>
          <div className="flex flex-wrap gap-2">
            <button onClick={() => void openBackupDirectoryBtn()} disabled={openingBackupDirectory}
              className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
              {openingBackupDirectory ? "Ouverture..." : "Ouvrir le dossier des backups"}
            </button>
            <button onClick={() => setShowBackupHelpModal(false)} disabled={openingBackupDirectory}
              className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">Fermer</button>
          </div>
        </div>
      </Modal>

      <Modal open={!!restorePromptBackup} onClose={() => { if (restoringBackup) return; setRestorePromptBackup(null); setRestorePassphrase(""); }} title="Restaurer un backup portable">
        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">Ce backup a été chiffré avec un mot de passe portable.</p>
          <input value={restorePassphrase} onChange={(e) => setRestorePassphrase(e.target.value)} type="password"
            placeholder="Mot de passe du backup portable" className="h-10 w-full rounded-xl border bg-background px-3 text-sm" />
          <div className="flex gap-2">
            <button onClick={() => restorePromptBackup && void restoreBackup(restorePromptBackup, restorePassphrase)}
              disabled={!restorePassphrase || !!restoringBackup}
              className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
              {restoringBackup ? "Restauration..." : "Restaurer"}
            </button>
            <button onClick={() => { setRestorePromptBackup(null); setRestorePassphrase(""); }} disabled={!!restoringBackup}
              className="rounded-xl border px-4 py-2 text-sm hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">Annuler</button>
          </div>
        </div>
      </Modal>
    </>
  );
}
