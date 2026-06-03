import { useState, useEffect } from "react";
import toast from "react-hot-toast";
import type { Structure, StructureInput, Categorie, AffiliationAvecDetails, Personne, Fonction } from "../types";
import { useAuth } from "../lib/auth";
import { invoke } from "../lib/tauri";
import { Modal } from "../components/Modal";
import { useEditLock } from "../hooks/useEditLock";
import { Field, Label, Icon } from "../lib/ui";
import { AffiliationModal } from "./AffiliationModal";

export function StructureModal({
  structure,
  onClose,
  categories = [],
  personnes = [],
  fonctions = [],
}: {
  structure: Structure;
  onClose: () => void;
  categories?: Categorie[];
  personnes?: Personne[];
  fonctions?: Fonction[];
}) {
  const { can } = useAuth();
  const [form, setForm] = useState<StructureInput>({
    id_structure: structure.id_structure || null,
    nom_structure: structure.nom_structure,
    adresse_structure: structure.adresse_structure,
    code_postal_structure: structure.code_postal_structure,
    commune_structure: structure.commune_structure,
    pays: structure.pays,
    telephone_general: structure.telephone_general,
    email_general: structure.email_general,
    site_web: structure.site_web,
    partenaire_direct: structure.partenaire_direct,
    notes_commentaires: structure.notes_commentaires,
    service_specifique: structure.service_specifique,
    reseau_subvention: structure.reseau_subvention,
    id_categorie: structure.id_categorie,
    original_updated_at: structure.updated_at,
  });
  const [affiliations, setAffiliations] = useState<AffiliationAvecDetails[]>([]);
  const [showAffModal, setShowAffModal] = useState(false);
  const [editAff, setEditAff] = useState<AffiliationAvecDetails | null>(null);
  const [duplicateStructure, setDuplicateStructure] = useState<Structure | null>(null);

  const canSave = can(structure.id_structure ? "structures.update" : "structures.create");
  const canDelete = can("structures.delete");
  const canReadAffiliations = can("affiliations.read");
  const canCreateAffiliation = can("affiliations.create") && !!structure.id_structure;
  const canEditAffiliation = can("affiliations.update");
  const canDeleteAffiliation = can("affiliations.delete");
  const { lockStatus, lockBlocked } = useEditLock("structures", structure.id_structure, canSave && !!structure.id_structure);
  const canMutateAffiliations = !lockBlocked;

  useEffect(() => {
    if (structure.id_structure) {
      invoke<AffiliationAvecDetails[]>("lister_affiliations_structure", { structureId: structure.id_structure })
        .then(setAffiliations)
        .catch(() => {});
    }
  }, [structure.id_structure]);

  const loadAffs = () => {
    if (structure.id_structure) {
      invoke<AffiliationAvecDetails[]>("lister_affiliations_structure", { structureId: structure.id_structure })
        .then(setAffiliations)
        .catch(() => {});
    }
  };

  const save = async () => {
    if (!canSave || lockBlocked) return;

    // Vérifier doublon email si création
    if (!structure.id_structure && form.email_general) {
      try {
        const existing = await invoke<Structure[]>("rechercher_structures_par_email", { email: form.email_general });
        if (existing.length > 0) {
          setDuplicateStructure(existing[0]);
          return;
        }
      } catch { /* ignorer */ }
    }

    doSave();
  };

  const confirmDuplicateSave = async () => {
    setDuplicateStructure(null);
    doSave();
  };

  const doSave = async () => {
    if (!canSave || lockBlocked) return;
    try {
      await invoke("sauvegarder_structure", { structure: form });
      toast.success("Structure enregistrée");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const del = async () => {
    if (!canDelete || !structure.id_structure || lockBlocked || !confirm("Supprimer cette structure ?")) return;
    try {
      await invoke("supprimer_structure", { id: structure.id_structure });
      toast.success("Structure supprimée");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <Modal open={true} onClose={onClose} title={`${structure.id_structure ? "Modifier" : "Nouvelle"} Structure`}>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Field label="Nom" value={form.nom_structure} onChange={(v) => setForm({ ...form, nom_structure: v || null })} disabled={!canSave} />
        <Field label="Adresse" value={form.adresse_structure} onChange={(v) => setForm({ ...form, adresse_structure: v || null })} disabled={!canSave} />
        <Field label="Code postal" value={form.code_postal_structure} onChange={(v) => setForm({ ...form, code_postal_structure: v || null })} disabled={!canSave} />
        <Field label="Commune" value={form.commune_structure} onChange={(v) => setForm({ ...form, commune_structure: v || null })} disabled={!canSave} />
        <Field label="Téléphone" value={form.telephone_general} onChange={(v) => setForm({ ...form, telephone_general: v || null })} disabled={!canSave} />
        <Field label="Email" value={form.email_general} onChange={(v) => setForm({ ...form, email_general: v || null })} disabled={!canSave} />
        <Field label="Service" value={form.service_specifique} onChange={(v) => setForm({ ...form, service_specifique: v || null })} disabled={!canSave} />
        <Field label="Site web" value={form.site_web} onChange={(v) => setForm({ ...form, site_web: v || null })} disabled={!canSave} />
        <div>
          <Label>Catégorie</Label>
          <select
            value={form.id_categorie ?? ""}
            onChange={(e) => setForm({ ...form, id_categorie: e.target.value ? Number(e.target.value) : null })}
            disabled={!canSave}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          >
            <option value="">— Aucune catégorie —</option>
            {categories.map((c) => (
              <option key={c.id_categorie} value={c.id_categorie}>
                {c.nom_categorie}
              </option>
            ))}
          </select>
        </div>
        <div className="md:col-span-2">
          <Label>Notes</Label>
          <textarea
            value={form.notes_commentaires ?? ""}
            onChange={(e) => setForm({ ...form, notes_commentaires: e.target.value || null })}
            disabled={!canSave}
            className="mt-1 h-20 w-full rounded-lg border bg-background px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          />
        </div>
      </div>

      {lockBlocked && (
        <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
          Cette structure est actuellement éditée par {lockStatus?.holder_label || "un autre utilisateur"}. L’enregistrement est bloqué tant que le verrou est actif.
        </div>
      )}

      {canReadAffiliations && (
        <div className="mt-6">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-semibold">Affiliations</h3>
            {canCreateAffiliation && (
              <button
                onClick={() => {
                  setEditAff(null);
                  setShowAffModal(true);
                }}
                disabled={!canMutateAffiliations}
                className="cursor-pointer rounded-lg border px-3 py-1 text-xs font-medium hover:bg-muted flex items-center gap-1 disabled:cursor-not-allowed disabled:opacity-60"
              >
                <Icon name="plus" className="size-3.5" /> Ajouter
              </button>
            )}
          </div>
          {affiliations.length === 0 ? (
            <p className="text-sm text-muted-foreground">Aucune affiliation</p>
          ) : (
            <div className="overflow-x-auto rounded-lg border">
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b bg-muted/50 text-left">
                    <th className="px-3 py-2 font-medium text-muted-foreground">Nom</th>
                    <th className="px-3 py-2 font-medium text-muted-foreground">Prénom</th>
                    <th className="px-3 py-2 font-medium text-muted-foreground">Fonction</th>
                    <th className="px-3 py-2 font-medium text-muted-foreground">Catégorie</th>
                    <th className="px-3 py-2 font-medium text-muted-foreground">Intitulé</th>
                    <th className="px-3 py-2 font-medium text-muted-foreground">Email pro</th>
                    <th className="px-3 py-2 font-medium text-muted-foreground">Tél. fixe pro</th>
                    <th className="px-3 py-2 font-medium text-muted-foreground">GSM pro</th>
                    <th className="px-3 py-2"></th>
                  </tr>
                </thead>
                <tbody>
                  {affiliations.map((a) => (
                    <tr key={a.id_affiliation} className="border-b last:border-0">
                      <td className="px-3 py-2 font-medium">{a.nom_personne || "—"}</td>
                      <td className="px-3 py-2">{a.prenom_personne || "—"}</td>
                      <td className="px-3 py-2">{a.libelle_fonction || "—"}</td>
                      <td className="px-3 py-2">{a.nom_categorie || "—"}</td>
                      <td className="px-3 py-2">{a.titre_specifique || "—"}</td>
                      <td className="px-3 py-2 text-muted-foreground">{a.email_professionnel || "—"}</td>
                      <td className="px-3 py-2 text-muted-foreground">{a.telephone_direct || "—"}</td>
                      <td className="px-3 py-2 text-muted-foreground">{a.gsm_professionnel || "—"}</td>
                      <td className="px-3 py-2 flex gap-1">
                        {canEditAffiliation && (
                          <button
                            onClick={() => {
                              setEditAff(a);
                              setShowAffModal(true);
                            }}
                            disabled={!canMutateAffiliations}
                            className="cursor-pointer text-muted-foreground hover:text-foreground disabled:cursor-not-allowed disabled:opacity-60"
                          >
                            <Icon name="edit" className="size-4" />
                          </button>
                        )}
                        {canDeleteAffiliation && (
                          <button
                            onClick={async () => {
                              if (!canMutateAffiliations) return;
                              try { await invoke("supprimer_affiliation", { id: a.id_affiliation }); } catch (e) { toast.error(String(e)); }
                              loadAffs();
                            }}
                            disabled={!canMutateAffiliations}
                            className="cursor-pointer text-red-500 hover:text-red-700 disabled:cursor-not-allowed disabled:opacity-60"
                          >
                            <Icon name="trash" className="size-4" />
                          </button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* Alerte doublon email structure */}
      {duplicateStructure && (
        <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50 p-4">
          <div className="flex items-start gap-3">
            <span className="text-xl">⚠️</span>
            <div className="flex-1">
              <p className="text-sm font-semibold text-amber-900">Une structure existe déjà avec cet email</p>
              <div className="mt-2 text-sm text-amber-800 space-y-0.5">
                <p><span className="font-medium">{duplicateStructure.nom_structure}</span></p>
                <p className="text-amber-700">{duplicateStructure.email_general}</p>
                <p className="text-amber-700">{duplicateStructure.telephone_general || "—"}</p>
              </div>
            </div>
          </div>
          <div className="mt-3 flex justify-end gap-2">
            <button onClick={() => setDuplicateStructure(null)}
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

      <div className="mt-6 flex justify-end gap-3">
        {!!structure.id_structure && canDelete && (
          <button disabled={lockBlocked} onClick={del} className="cursor-pointer rounded-xl border border-red-200 bg-red-50 px-4 py-2 text-sm font-medium text-red-700 hover:bg-red-100 disabled:cursor-not-allowed disabled:opacity-60">
            Supprimer
          </button>
        )}
        <button onClick={onClose} className="cursor-pointer rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted">
          Annuler
        </button>
        {canSave && (
          <button disabled={lockBlocked} onClick={save} className="cursor-pointer rounded-xl bg-primary px-6 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60">
            Enregistrer
          </button>
        )}
      </div>

      {showAffModal && canReadAffiliations && (
        <AffiliationModal
          structureId={structure.id_structure}
          personnes={personnes}
          structures={[]}
          fonctions={fonctions}
          categories={categories}
          existing={editAff}
          onClose={async () => {
            setShowAffModal(false);
            setEditAff(null);
            loadAffs();
          }}
        />
      )}
    </Modal>
  );
}
