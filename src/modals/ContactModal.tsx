import { useState, useEffect, useMemo } from "react";
import toast from "react-hot-toast";
import type { Personne, PersonneInput, Categorie, AffiliationAvecDetails, Structure, Fonction } from "../types";
import { useAuth } from "../lib/auth";
import { invoke } from "../lib/tauri";
import { Modal } from "../components/Modal";
import { normalizeLastNameInput } from "../lib/format";
import { useEditLock } from "../hooks/useEditLock";
import { Icon, Label, Field } from "../lib/ui";
import { AffiliationModal } from "./AffiliationModal";
import { useAffiliations } from "../hooks/useAffiliations";
import { AffiliationsTable } from "../components/AffiliationsTable";

const CONTACT_STATUSES = [
  { value: "Actif", label: "Actif" },
  { value: "A supprimer", label: "A supprimer" },
] as const;

export function ContactModal({ personne, onClose, categories }: { personne: Personne; onClose: () => void; categories: Categorie[] }) {
  const { can } = useAuth();
  const [form, setForm] = useState<PersonneInput>({
    id_personne: personne.id_personne || null,
    civilite: personne.civilite, nom: personne.nom, prenom: personne.prenom,
    email_prive: personne.email_prive, telephone_prive: personne.telephone_prive,
    adresse_privee: personne.adresse_privee, code_postal_prive: personne.code_postal_prive,
    commune_privee: personne.commune_privee, pays: personne.pays ?? "Belgique",
    consentement_rgpd: personne.consentement_rgpd, statut_compte: personne.statut_compte,
    notes_commentaires: personne.notes_commentaires,
    original_updated_at: personne.updated_at,
  });
  const [structures, setStructures] = useState<Structure[]>([]);
  const [fonctions, setFonctions] = useState<Fonction[]>([]);
  const [showAffModal, setShowAffModal] = useState(false);
  const [editAff, setEditAff] = useState<AffiliationAvecDetails | null>(null);
  const [duplicatePerson, setDuplicatePerson] = useState<Personne | null>(null);
  const canSavePerson = can(personne.id_personne ? "personnes.update" : "personnes.create");
  const canDeletePerson = can("personnes.delete");
  const canReadAffiliations = can("affiliations.read");
  const canCreateAffiliation = can("affiliations.create") && !!personne.id_personne;
  const canEditAffiliation = can("affiliations.update");
  const canDeleteAffiliation = can("affiliations.delete");
  const { lockStatus, lockBlocked } = useEditLock("personnes", personne.id_personne, canSavePerson && !!personne.id_personne);
  const canMutateAffiliations = !lockBlocked;
  const affiliationOwner = useMemo(
    () => personne.id_personne ? { personneId: personne.id_personne } : null,
    [personne.id_personne],
  );
  const { data: affiliations, reload: loadAffs } = useAffiliations(affiliationOwner);
  useEffect(() => {
    void Promise.allSettled([
      invoke<Structure[]>("lister_structures"),
      invoke<Fonction[]>("lister_fonctions"),
    ]).then(([nextStructures, nextFunctions]) => {
      setStructures(nextStructures.status === "fulfilled" ? nextStructures.value : []);
      setFonctions(nextFunctions.status === "fulfilled" ? nextFunctions.value : []);
    });
  }, []);

  const save = async () => {
    if (!canSavePerson || lockBlocked) return;

    // Vérifier doublon email si création
    if (!personne.id_personne && form.email_prive) {
      try {
        const existing = await invoke<Personne[]>("rechercher_personnes_par_email", { email: form.email_prive });
        if (existing.length > 0) {
          setDuplicatePerson(existing[0]);
          return;
        }
      } catch { /* ignorer, on laisse sauvegarder */ }
    }

    try {
      await invoke("sauvegarder_personne", { personne: form });
      toast.success("Contact enregistré");
      onClose();
    } catch (e) { toast.error(String(e)); }
  };

  const confirmDuplicateSave = async () => {
    setDuplicatePerson(null);
    try {
      await invoke("sauvegarder_personne", { personne: form });
      toast.success("Contact enregistré");
      onClose();
    } catch (e) { toast.error(String(e)); }
  };
  const del = async () => {
    if (!canDeletePerson || !personne.id_personne || lockBlocked || !confirm("Supprimer ce contact ?")) return;
    try {
      await invoke("supprimer_personne", { id: personne.id_personne });
      toast.success("Contact supprimé");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <Modal open={true} onClose={onClose} title={`${personne.id_personne ? "Modifier" : "Nouveau"} Contact`}>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <div>
          <Label>Civilité</Label>
          <select value={form.civilite ?? ""} onChange={(e) => setForm({ ...form, civilite: e.target.value || null })}
            disabled={!canSavePerson}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60">
            <option value="">—</option><option value="M">M.</option><option value="Mme">Mme</option><option value="Mlle">Mlle</option>
          </select>
        </div>
        <Field label="Nom" value={form.nom} onChange={(v) => setForm({ ...form, nom: normalizeLastNameInput(v) || null })} disabled={!canSavePerson} />
        <Field label="Prénom" value={form.prenom} onChange={(v) => setForm({ ...form, prenom: v || null })} disabled={!canSavePerson} />
        <Field label="Email privé" value={form.email_prive} onChange={(v) => setForm({ ...form, email_prive: v || null })} disabled={!canSavePerson} />
        <Field label="Téléphone privé" value={form.telephone_prive} onChange={(v) => setForm({ ...form, telephone_prive: v || null })} disabled={!canSavePerson} />
        <Field label="Adresse" value={form.adresse_privee} onChange={(v) => setForm({ ...form, adresse_privee: v || null })} className="md:col-span-2" disabled={!canSavePerson} />
        <Field label="Code postal" value={form.code_postal_prive} onChange={(v) => setForm({ ...form, code_postal_prive: v || null })} disabled={!canSavePerson} />
        <Field label="Commune" value={form.commune_privee} onChange={(v) => setForm({ ...form, commune_privee: v || null })} disabled={!canSavePerson} />
        <div>
          <Label>Statut</Label>
          <select
            value={form.statut_compte ?? ""}
            onChange={(e) => setForm({ ...form, statut_compte: e.target.value || null })}
            disabled={!canSavePerson || personne.statut_compte === "Anonymisé"}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          >
            <option value="">— Aucun statut —</option>
            {CONTACT_STATUSES.map((status) => (
              <option key={status.value} value={status.value}>
                {status.label}
              </option>
            ))}
            {personne.statut_compte === "Anonymisé" && <option value="Anonymisé">Anonymisé</option>}
          </select>
        </div>
        <div className="md:col-span-2">
          <Label>Notes</Label>
          <textarea value={form.notes_commentaires ?? ""} onChange={(e) => setForm({ ...form, notes_commentaires: e.target.value || null })}
            disabled={!canSavePerson}
            className="mt-1 h-20 w-full rounded-lg border bg-background px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-60" />
        </div>
        <div className="flex items-center gap-3 md:col-span-2">
          <label className="flex items-center gap-2 text-sm cursor-pointer">
            <input type="checkbox" checked={form.consentement_rgpd} onChange={(e) => setForm({ ...form, consentement_rgpd: e.target.checked })}
              disabled={!canSavePerson}
              className="rounded border-gray-300 text-primary focus:ring-primary" />
            Consentement RGPD
          </label>
        </div>
      </div>

      {lockBlocked && (
        <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
          Cette fiche est actuellement éditée par {lockStatus?.holder_label || "un autre utilisateur"}. Passe en lecture seule pour éviter un écrasement.
        </div>
      )}

      {canReadAffiliations && (
      <div className="mt-6">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold">Affiliations</h3>
          {canCreateAffiliation && <button onClick={() => { setEditAff(null); setShowAffModal(true); }}
            disabled={!canMutateAffiliations}
            className="cursor-pointer rounded-lg border px-3 py-1 text-xs font-medium hover:bg-muted flex items-center gap-1 disabled:cursor-not-allowed disabled:opacity-60">
            <Icon name="plus" className="size-3.5" /> Ajouter
          </button>}
        </div>
        <AffiliationsTable affiliations={affiliations} perspective="personne"
          canEdit={canEditAffiliation} canDelete={canDeleteAffiliation} disabled={!canMutateAffiliations}
          onEdit={(affiliation) => { setEditAff(affiliation); setShowAffModal(true); }}
          onDelete={async (affiliation) => {
            try {
              await invoke("supprimer_affiliation", { id: affiliation.id_affiliation });
              await loadAffs();
            } catch (error) {
              toast.error(String(error));
            }
          }} />
      </div>
      )}

      <div className="mt-6 flex justify-end gap-3">
        {!!personne.id_personne && canDeletePerson && <button disabled={lockBlocked} onClick={del} className="cursor-pointer rounded-xl border border-red-200 bg-red-50 px-4 py-2 text-sm font-medium text-red-700 hover:bg-red-100 disabled:cursor-not-allowed disabled:opacity-60">Supprimer</button>}
        <button onClick={onClose} className="cursor-pointer rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted">Annuler</button>
        {canSavePerson && <button disabled={lockBlocked} onClick={save} className="cursor-pointer rounded-xl bg-primary px-6 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60">Enregistrer</button>}
      </div>

      {/* Alerte doublon email */}
      {duplicatePerson && (
        <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50 p-4">
          <div className="flex items-start gap-3">
            <span className="text-xl">⚠️</span>
            <div className="flex-1">
              <p className="text-sm font-semibold text-amber-900">Une personne existe déjà avec cet email</p>
              <div className="mt-2 text-sm text-amber-800 space-y-0.5">
                <p><span className="font-medium">{duplicatePerson.prenom || ""} {duplicatePerson.nom || ""}</span></p>
                <p className="text-amber-700">{duplicatePerson.email_prive}</p>
                <p className="text-amber-700">{duplicatePerson.telephone_prive || "—"}</p>
                <p className="text-amber-700">Statut : {duplicatePerson.statut_compte || "—"}</p>
              </div>
            </div>
          </div>
          <div className="mt-3 flex justify-end gap-2">
            <button onClick={() => setDuplicatePerson(null)}
              className="cursor-pointer rounded-lg border bg-white px-3 py-1.5 text-xs font-medium hover:bg-amber-100">
              Annuler
            </button>
            <button onClick={confirmDuplicateSave}
              className="cursor-pointer rounded-lg bg-amber-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-amber-700">
              Ajouter quand même
            </button>
          </div>
        </div>
      )}

      {showAffModal && canReadAffiliations && <AffiliationModal personneId={personne.id_personne!} structures={structures} fonctions={fonctions} categories={categories}
        existing={editAff} onClose={async () => { setShowAffModal(false); setEditAff(null); await loadAffs().catch(() => {}); }} />}
    </Modal>
  );
}
