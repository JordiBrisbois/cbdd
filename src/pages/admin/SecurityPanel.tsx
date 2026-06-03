import { useState } from "react";
import toast from "react-hot-toast";
import type { SecuritySettings } from "../../types";
import { invoke } from "../../lib/tauri";
import { DEFAULT_UPDATE_ENDPOINT } from "./constants";

export function SecurityPanel({
  settings, setSettings,
}: {
  settings: SecuritySettings;
  setSettings: (s: SecuritySettings) => void;
}) {
  const [customUpdateEndpoint, setCustomUpdateEndpoint] = useState(() =>
    typeof window === "undefined" ? "" : localStorage.getItem("crvi-update-endpoint") || ""
  );

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

  return (
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
          Laisse vide pour utiliser l'URL de release par défaut. Ce réglage est local à ce poste.
        </p>
      </div>
      <div className="mt-4">
        <button onClick={() => void saveSettings()} className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer">
          Enregistrer la sécurité
        </button>
      </div>
    </div>
  );
}
