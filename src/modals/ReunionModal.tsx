import { useState, useEffect, useMemo } from "react";
import toast from "react-hot-toast";
import type { Reunion, PresenceAvecDetails, Personne, ReunionInput, Structure } from "../types";
import { invoke, Modal, fullName, exportTableFile, useEditLock } from "../lib/utils";
import { Icon, Label, Field } from "../lib/ui";
import { TableExportModal, type TableExportFormat, type TableExportScope } from "../components/TableExportModal";
import { useAuth } from "../lib/auth";

const STATUTS = ["Présent", "Excusé", "Absent", "Invité"] as const;

export function ReunionModal({ reunion, onClose }: { reunion: Reunion; onClose: () => void }) {
  const { can } = useAuth();
  const [form, setForm] = useState<ReunionInput>({
    id_reunion: reunion.id_reunion || null,
    titre_reunion: reunion.titre_reunion ?? "",
    date_reunion: reunion.date_reunion ?? "",
    heure_reunion: reunion.heure_reunion ?? "",
    lieu_reunion: reunion.lieu_reunion,
    ref_structure: reunion.ref_structure ?? (reunion.id_reunion ? null : 113),
    notes_commentaires: reunion.notes_commentaires,
    original_updated_at: reunion.updated_at,
  });
  const [presences, setPresences] = useState<PresenceAvecDetails[]>([]);
  const [personnes, setPersonnes] = useState<Personne[]>([]);
  const [structures, setStructures] = useState<Structure[]>([]);
  const [addPersonneId, setAddPersonneId] = useState<number | null>(null);
  const [addPersonSearch, setAddPersonSearch] = useState("");
  const [addRgpd, setAddRgpd] = useState(true);
  const [addStatut, setAddStatut] = useState("Présent");
  const [showExportModal, setShowExportModal] = useState(false);
  const showPresences = !!reunion.id_reunion;
  const { lockStatus, lockBlocked } = useEditLock("reunions", reunion.id_reunion, !!reunion.id_reunion);
  const canReadStructures = can("structures.read");
  const canUpdateReunion = reunion.id_reunion ? can("reunions.update") : can("reunions.create");
  const canDeleteReunion = !!reunion.id_reunion && can("reunions.delete");
  const canReadPresences = can("presences.read");
  const canCreatePresence = can("presences.create");
  const canUpdatePresence = can("presences.update");
  const canDeletePresence = can("presences.delete");
  const readOnlyReunion = lockBlocked || !canUpdateReunion;
  const existingPresenceIds = useMemo(
    () => new Set(presences.map((presence) => presence.ref_personne).filter((id): id is number => typeof id === "number")),
    [presences],
  );
  const addPersonSearchReady = addPersonSearch.trim().length >= 2;
  const filteredPersonnes = useMemo(() => {
    const needle = addPersonSearch.trim().toLowerCase();
    return personnes
      .filter((personne) => !existingPresenceIds.has(personne.id_personne))
      .filter((personne) => {
        if (!needle) return true;
        const nom = (personne.nom ?? "").toLowerCase();
        const prenom = (personne.prenom ?? "").toLowerCase();
        const email = (personne.email_prive ?? "").toLowerCase();
        return nom.includes(needle) || prenom.includes(needle) || `${nom} ${prenom}`.includes(needle) || `${prenom} ${nom}`.includes(needle) || email.includes(needle);
      })
      .sort((a, b) => {
        const nomCompare = (a.nom ?? "").localeCompare(b.nom ?? "", "fr", { sensitivity: "base" });
        if (nomCompare !== 0) return nomCompare;
        return (a.prenom ?? "").localeCompare(b.prenom ?? "", "fr", { sensitivity: "base" });
      })
      .slice(0, 5);
  }, [addPersonSearch, existingPresenceIds, personnes]);
  const selectedPersonne = useMemo(
    () => personnes.find((personne) => personne.id_personne === addPersonneId) ?? null,
    [addPersonneId, personnes],
  );

  const formatParticipantOption = (personne: Personne) => {
    const nom = (personne.nom ?? "").trim().toUpperCase();
    const prenom = (personne.prenom ?? "").trim();
    return [nom, prenom].filter(Boolean).join(" ") || fullName(personne);
  };

  const loadPresences = () => {
    if (reunion.id_reunion && canReadPresences) {
      invoke<PresenceAvecDetails[]>("lister_presences_reunion", { reunionId: reunion.id_reunion }).then(setPresences).catch((e) => toast.error(String(e)));
    }
  };

  useEffect(() => { loadPresences(); }, [reunion.id_reunion]);
  useEffect(() => {
    if (!canReadStructures) return;
    invoke<Structure[]>("lister_structures").then(setStructures).catch((e) => toast.error(String(e)));
  }, [canReadStructures]);

  const toggleRgpd = async (p: PresenceAvecDetails, value: boolean) => {
    if (lockBlocked || !canUpdatePresence) return;
    try {
      await invoke("sauvegarder_presence", {
        presence: { id_presence: p.id_presence, ref_reunion: reunion.id_reunion, ref_personne: p.ref_personne, statut_presence: p.statut_presence, souhaite_rester_en_bdd: value, notes_commentaires: p.notes_commentaires }
      });
      loadPresences();
    } catch (e) { toast.error(String(e)); }
  };

  const updateStatut = async (p: PresenceAvecDetails, statut: string) => {
    if (lockBlocked || !canUpdatePresence) return;
    try {
      await invoke("sauvegarder_presence", {
        presence: { id_presence: p.id_presence, ref_reunion: reunion.id_reunion, ref_personne: p.ref_personne, statut_presence: statut, souhaite_rester_en_bdd: p.souhaite_rester_en_bdd, notes_commentaires: p.notes_commentaires }
      });
      loadPresences();
    } catch (e) { toast.error(String(e)); }
  };

  const addPresence = async () => {
    if (!addPersonneId || !reunion.id_reunion || lockBlocked || !canCreatePresence) return;
    try {
      await invoke("sauvegarder_presence", {
        presence: { ref_reunion: reunion.id_reunion, ref_personne: addPersonneId, statut_presence: addStatut, souhaite_rester_en_bdd: addRgpd }
      });
      toast.success("Participant ajouté");
      setAddPersonneId(null);
      setAddPersonSearch("");
      setAddRgpd(true);
      setAddStatut("Présent");
      loadPresences();
    } catch (e) { toast.error(String(e)); }
  };

  const save = async () => {
    if (lockBlocked || !canUpdateReunion) return;
    try {
      await invoke("sauvegarder_reunion", { reunion: form });
      toast.success("Réunion enregistrée");
      onClose();
    } catch (e) { toast.error(String(e)); }
  };
  const del = async () => {
    if (!reunion.id_reunion || lockBlocked || !canDeleteReunion || !confirm("Supprimer cette réunion ?")) return;
    try {
      await invoke("supprimer_reunion", { id: reunion.id_reunion });
      toast.success("Réunion supprimée");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const exportPresences = (scope: TableExportScope, format: TableExportFormat) => {
    const currentHeaders = ["Participant", "Statut", "Souhaite rester en BDD"];
    const currentRows = presences.map(p => [
      `${p.prenom_personne ?? ""} ${p.nom_personne ?? ""}`.trim(),
      p.statut_presence ?? "",
      p.souhaite_rester_en_bdd ? "Oui" : "Non",
    ]);
    const rawHeaders = ["ID présence", "Ref personne", "Nom", "Prénom", "Statut", "Souhaite rester en BDD", "Notes"];
    const rawRows = presences.map((p) => [
      String(p.id_presence ?? ""),
      String(p.ref_personne ?? ""),
      p.nom_personne ?? "",
      p.prenom_personne ?? "",
      p.statut_presence ?? "",
      p.souhaite_rester_en_bdd ? "Oui" : "Non",
      p.notes_commentaires ?? "",
    ]);
    const headers = scope === "current" ? currentHeaders : rawHeaders;
    const rows = scope === "current" ? currentRows : rawRows;
    exportTableFile(headers, rows, `presences_reunion_${reunion.titre_reunion ?? "reunion"}_${new Date().toISOString().slice(0, 10)}`, format);
    toast.success(`${presences.length} présences exportées`);
  };

  return (
    <Modal open={true} onClose={onClose} title={`${reunion.id_reunion ? "Modifier" : "Nouvelle"} Réunion`}>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Field label="Titre" value={form.titre_reunion} onChange={(v) => setForm({ ...form, titre_reunion: v })} />
        <div>
          <Label>Organisme</Label>
          {canReadStructures ? (
            <select value={form.ref_structure ?? ""} onChange={(e) => setForm({ ...form, ref_structure: e.target.value ? Number(e.target.value) : null })}
              disabled={readOnlyReunion}
              className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm focus:border-ring focus:ring-2 focus:ring-ring/30 disabled:cursor-not-allowed disabled:opacity-60">
              <option value="">—</option>
              {structures.map((s) => <option key={s.id_structure} value={s.id_structure}>{s.nom_structure}</option>)}
            </select>
          ) : (
            <div className="mt-1 flex h-9 items-center rounded-lg border bg-muted/30 px-3 text-sm text-muted-foreground">
              {reunion.nom_structure || "—"}
            </div>
          )}
        </div>
        <Field label="Lieu" value={form.lieu_reunion ?? ""} onChange={(v) => setForm({ ...form, lieu_reunion: v || null })} disabled={readOnlyReunion} />
        <Field label="Date" value={form.date_reunion} onChange={(v) => setForm({ ...form, date_reunion: v })} type="date" disabled={readOnlyReunion} />
        <Field label="Heure" value={form.heure_reunion} onChange={(v) => setForm({ ...form, heure_reunion: v })} type="time" disabled={readOnlyReunion} />
        <div className="md:col-span-2">
          <Label>Notes</Label>
          <textarea value={form.notes_commentaires ?? ""} onChange={(e) => setForm({ ...form, notes_commentaires: e.target.value || null })}
            disabled={readOnlyReunion}
            className="mt-1 h-20 w-full rounded-lg border bg-background px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-60" />
        </div>
      </div>

      {lockBlocked && (
        <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
          Cette réunion est actuellement éditée par {lockStatus?.holder_label || "un autre utilisateur"}. Les modifications et les présences passent en lecture seule tant que le verrou est actif.
        </div>
      )}

      {showPresences && canReadPresences && (
        <div className="mt-6">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold">Présences ({presences.length})</h3>
            <div className="flex gap-2">
              {presences.length > 0 && (
                <button onClick={() => setShowExportModal(true)} className="flex items-center gap-1 rounded-lg border px-2 py-1 text-xs font-medium hover:bg-muted cursor-pointer">
                  <Icon name="download" className="size-3.5" /> Exporter
                </button>
              )}
              <button onClick={() => { if (!personnes.length) invoke<Personne[]>("lister_personnes").then(setPersonnes).catch((e) => toast.error(String(e))); }}
                disabled={lockBlocked || !canCreatePresence}
                className="flex items-center gap-1 rounded-lg border px-3 py-1 text-xs font-medium hover:bg-muted cursor-pointer disabled:cursor-not-allowed disabled:opacity-60">
                <Icon name="plus" className="size-3.5" /> Ajouter
              </button>
            </div>
          </div>
          {presences.length === 0 ? (
            <p className="text-sm text-muted-foreground">Aucune présence enregistrée</p>
          ) : (
            <div className="overflow-x-auto rounded-lg border">
              <table className="w-full text-xs">
                <thead><tr className="border-b bg-muted/50 text-left">
                  <th className="px-3 py-2 font-medium text-muted-foreground">Participant</th>
                  <th className="px-3 py-2 font-medium text-muted-foreground">Statut</th>
                  <th className="px-3 py-2 font-medium text-muted-foreground">Reste en BDD</th>
                  <th className="px-3 py-2"></th>
                </tr></thead>
                <tbody>
                  {presences.map((p) => (
                    <tr key={p.id_presence} className="border-b last:border-0">
                      <td className="px-3 py-2 font-medium">{p.prenom_personne || ""} {p.nom_personne || ""}</td>
                      <td className="px-3 py-2">
                        <select value={p.statut_presence || "Présent"} onChange={(e) => updateStatut(p, e.target.value)}
                          disabled={lockBlocked || !canUpdatePresence}
                          className="h-7 rounded border bg-background px-1 text-xs disabled:cursor-not-allowed disabled:opacity-60">
                          {STATUTS.map((s) => <option key={s} value={s}>{s}</option>)}
                        </select>
                      </td>
                      <td className="px-3 py-2">
                        <label className="flex cursor-pointer items-center gap-1.5">
                          <input type="checkbox" checked={p.souhaite_rester_en_bdd} onChange={(e) => toggleRgpd(p, e.target.checked)}
                            disabled={lockBlocked || !canUpdatePresence}
                            className="rounded border-gray-300 text-primary focus:ring-primary" />
                          <span className="text-xs">{p.souhaite_rester_en_bdd ? "Oui" : "Non"}</span>
                        </label>
                      </td>
                      <td className="px-3 py-2">
                        <button onClick={async () => {
                          if (lockBlocked || !canDeletePresence) return;
                          await invoke("supprimer_presence", { id: p.id_presence }).catch((e) => toast.error(String(e)));
                          loadPresences();
                        }} disabled={lockBlocked || !canDeletePresence} className="cursor-pointer text-red-500 hover:text-red-700 disabled:cursor-not-allowed disabled:opacity-60"><Icon name="trash" className="size-4" /></button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {personnes.length > 0 && (
            <div className="mt-4 rounded-lg border bg-muted/30 p-3">
              <div className="mb-3 text-sm font-semibold text-muted-foreground">Ajouter un participant</div>
              <div className="grid gap-4 xl:grid-cols-[minmax(0,1.6fr)_320px] xl:items-start">
                <div className="min-w-[240px]">
                  <Label>Participant</Label>
                  <input
                    value={addPersonSearch}
                    onChange={(e) => {
                      setAddPersonSearch(e.target.value);
                      setAddPersonneId(null);
                    }}
                    placeholder="Rechercher par NOM, prénom ou email..."
                    disabled={lockBlocked || !canCreatePresence}
                    className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
                  />
                  <div className="mt-1 flex items-center justify-between text-xs text-muted-foreground">
                    <span>{addPersonSearchReady ? `${filteredPersonnes.length} proposition(s)` : "Tape au moins 2 caractères pour rechercher."}</span>
                    {selectedPersonne && <span>Sélection en cours</span>}
                  </div>
                  {(addPersonSearchReady || !!selectedPersonne) && (
                  <div className="mt-2 max-h-64 overflow-y-auto rounded-xl border bg-background shadow-sm">
                    {filteredPersonnes.length > 0 ? (
                      filteredPersonnes.map((personne) => {
                        const isSelected = personne.id_personne === addPersonneId;
                        return (
                          <button
                            key={personne.id_personne}
                            type="button"
                            onClick={() => {
                              setAddPersonneId(personne.id_personne);
                              setAddPersonSearch(formatParticipantOption(personne));
                            }}
                            disabled={lockBlocked || !canCreatePresence}
                            className={`flex w-full items-start justify-between gap-3 border-b border-border/60 px-4 py-3 text-left text-sm transition-colors last:border-b-0 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60 ${isSelected ? "bg-primary/10 text-primary" : "hover:bg-muted/60"}`}
                          >
                            <div className="min-w-0">
                              <div className="truncate font-medium">{formatParticipantOption(personne)}</div>
                              <div className="truncate text-xs text-muted-foreground">{personne.email_prive || "Aucun email"}</div>
                            </div>
                            {isSelected && <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[11px] font-semibold">Sélectionné</span>}
                          </button>
                        );
                      })
                    ) : (
                      <div className="px-3 py-4 text-sm text-muted-foreground">
                        {personnes.some((personne) => !existingPresenceIds.has(personne.id_personne))
                          ? "Aucun participant ne correspond à la recherche."
                          : "Toutes les personnes disponibles sont déjà ajoutées à cette réunion."}
                      </div>
                    )}
                  </div>
                  )}
                </div>
                <div className="rounded-xl border bg-background p-4 shadow-sm">
                  <div>
                    <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Sélection</div>
                    <div className="mt-2 rounded-lg border bg-muted/40 px-3 py-3">
                      {selectedPersonne ? (
                        <>
                          <div className="font-medium">{formatParticipantOption(selectedPersonne)}</div>
                          <div className="mt-1 text-xs text-muted-foreground">{selectedPersonne.email_prive || "Aucun email"}</div>
                        </>
                      ) : (
                        <div className="text-sm text-muted-foreground">Choisis un participant dans la liste.</div>
                      )}
                    </div>
                  </div>

                  <div className="mt-4 space-y-4">
                    <div>
                      <Label>Statut</Label>
                      <select value={addStatut} onChange={(e) => setAddStatut(e.target.value)}
                        disabled={lockBlocked || !canCreatePresence}
                        className="mt-1 h-10 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60">
                        {STATUTS.map((s) => <option key={s} value={s}>{s}</option>)}
                      </select>
                    </div>

                    <label className="flex min-h-10 items-center gap-2 rounded-lg border bg-muted/30 px-3">
                      <input type="checkbox" checked={addRgpd} onChange={(e) => setAddRgpd(e.target.checked)}
                        disabled={lockBlocked || !canCreatePresence}
                        className="rounded border-gray-300 text-primary focus:ring-primary" />
                      <span className="text-sm">Reste en BDD</span>
                    </label>
                  </div>

                  <button onClick={addPresence} disabled={!addPersonneId || lockBlocked || !canCreatePresence}
                    className="mt-4 w-full cursor-pointer rounded-xl bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed">
                    Ajouter le participant
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      <div className="mt-6 flex justify-end gap-3">
        {!!reunion.id_reunion && canDeleteReunion && <button disabled={lockBlocked} onClick={del} className="cursor-pointer rounded-xl border border-red-200 bg-red-50 px-4 py-2 text-sm font-medium text-red-700 hover:bg-red-100 disabled:cursor-not-allowed disabled:opacity-60">Supprimer</button>}
        <button onClick={onClose} className="cursor-pointer rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted">Annuler</button>
        {canUpdateReunion && <button disabled={lockBlocked} onClick={save} className="cursor-pointer rounded-xl bg-primary px-6 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60">Enregistrer</button>}
      </div>
      <TableExportModal
        open={showExportModal}
        onClose={() => setShowExportModal(false)}
        onConfirm={exportPresences}
        rawDescription="Exporte toutes les donnees de presence disponibles pour cette reunion, y compris les notes."
      />
    </Modal>
  );
}
