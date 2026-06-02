import { useMemo, useState } from "react";
import toast from "react-hot-toast";
import type { AffiliationInput, AffiliationAvecDetails, PersonneInput, Structure, Fonction, Categorie, Personne } from "../types";
import { useAuth } from "../lib/auth";
import { fullName, invoke, Modal, normalizeLastNameInput, useEditLock } from "../lib/utils";
import { Label } from "../lib/ui";

export function AffiliationModal({
  personneId,
  structureId,
  personnes,
  structures,
  fonctions,
  categories,
  existing,
  onClose,
}: {
  personneId?: number | null;
  structureId?: number | null;
  personnes?: Personne[];
  structures: Structure[];
  fonctions: Fonction[];
  categories: Categorie[];
  existing: AffiliationAvecDetails | null;
  onClose: () => void;
}) {
  const { can } = useAuth();
  const isModeStructure = structureId != null;
  const isModePersonne = personneId != null;
  const availablePersonnes = personnes ?? [];

  const [form, setForm] = useState<AffiliationInput>({
    id_affiliation: existing?.id_affiliation ?? null,
    ref_personne: existing?.ref_personne ?? personneId ?? null,
    ref_structure: existing?.ref_structure ?? structureId ?? null,
    ref_fonction: existing?.ref_fonction ?? null,
    titre_specifique: existing?.titre_specifique ?? null,
    email_professionnel: existing?.email_professionnel ?? null,
    telephone_direct: existing?.telephone_direct ?? null,
    gsm_professionnel: existing?.gsm_professionnel ?? null,
    id_categorie: existing?.id_categorie ?? null,
    original_updated_at: existing?.updated_at ?? null,
  });

  const [showQuickAdd, setShowQuickAdd] = useState(false);
  const [duplicatePerson, setDuplicatePerson] = useState<Personne | null>(null);
  const [personSearch, setPersonSearch] = useState(() =>
    existing?.nom_personne || existing?.prenom_personne
      ? [existing?.nom_personne, existing?.prenom_personne].filter(Boolean).join(" ")
      : ""
  );
  const [structureSearch, setStructureSearch] = useState(() => existing?.nom_structure ?? "");
  const [newPerson, setNewPerson] = useState<PersonneInput>({
    civilite: null,
    nom: "",
    prenom: "",
    email_prive: null,
    telephone_prive: null,
    consentement_rgpd: false,
  });

  const canSave = can(existing ? "affiliations.update" : "affiliations.create");
  const canCreatePerson = can("personnes.create");
  const { lockStatus, lockBlocked } = useEditLock("affiliations", existing?.id_affiliation ?? null, canSave && !!existing?.id_affiliation);

  const formatPersonOption = (personne: Personne) => {
    const nom = (personne.nom ?? "").trim().toUpperCase();
    const prenom = (personne.prenom ?? "").trim();
    return [nom, prenom].filter(Boolean).join(" ") || fullName(personne);
  };

  const selectedPersonne = useMemo(
    () => availablePersonnes.find((personne) => personne.id_personne === form.ref_personne) ?? null,
    [availablePersonnes, form.ref_personne]
  );
  const selectedStructure = useMemo(
    () => structures.find((structure) => structure.id_structure === form.ref_structure) ?? null,
    [form.ref_structure, structures]
  );
  const personSearchReady = personSearch.trim().length >= 2;
  const structureSearchReady = structureSearch.trim().length >= 2;
  const filteredPersonnes = useMemo(() => {
    const needle = personSearch.trim().toLocaleLowerCase("fr-BE");
    return availablePersonnes
      .filter((personne) => {
        if (!needle) return true;
        const nom = (personne.nom ?? "").toLocaleLowerCase("fr-BE");
        const prenom = (personne.prenom ?? "").toLocaleLowerCase("fr-BE");
        const email = (personne.email_prive ?? "").toLocaleLowerCase("fr-BE");
        return nom.includes(needle)
          || prenom.includes(needle)
          || `${nom} ${prenom}`.includes(needle)
          || `${prenom} ${nom}`.includes(needle)
          || email.includes(needle);
      })
      .sort((a, b) => {
        const byNom = (a.nom ?? "").localeCompare(b.nom ?? "", "fr", { sensitivity: "base" });
        if (byNom !== 0) return byNom;
        return (a.prenom ?? "").localeCompare(b.prenom ?? "", "fr", { sensitivity: "base" });
      })
      .slice(0, 5);
  }, [availablePersonnes, personSearch]);
  const filteredStructures = useMemo(() => {
    const needle = structureSearch.trim().toLocaleLowerCase("fr-BE");
    return structures
      .filter((structure) => {
        if (!needle) return true;
        const nom = (structure.nom_structure ?? "").toLocaleLowerCase("fr-BE");
        const commune = (structure.commune_structure ?? "").toLocaleLowerCase("fr-BE");
        return nom.includes(needle) || commune.includes(needle);
      })
      .sort((a, b) => (a.nom_structure ?? "").localeCompare(b.nom_structure ?? "", "fr", { sensitivity: "base" }))
      .slice(0, 5);
  }, [structureSearch, structures]);

  const save = async () => {
    if (!canSave || lockBlocked) return;
    try {
      await invoke("sauvegarder_affiliation", { aff: form });
      toast.success(existing ? "Affiliation modifiée" : "Affiliation ajoutée");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const createPersonAndAffiliation = async () => {
    if (!canCreatePerson || !structureId) return;
    if (!newPerson.nom || !newPerson.prenom) {
      toast.error("Nom et Prénom sont obligatoires");
      return;
    }

    // Vérifier doublon email
    if (newPerson.email_prive) {
      try {
        const existing = await invoke<Personne[]>("rechercher_personnes_par_email", { email: newPerson.email_prive });
        if (existing.length > 0) {
          setDuplicatePerson(existing[0]);
          return;
        }
      } catch (e) { /* ignorer */ }
    }

    doCreatePersonAndAffiliation();
  };

  const confirmDuplicateCreate = async () => {
    setDuplicatePerson(null);
    doCreatePersonAndAffiliation();
  };

  const doCreatePersonAndAffiliation = async () => {
    try {
      const created = await invoke<Personne>("sauvegarder_personne", { personne: newPerson });
      await invoke("sauvegarder_affiliation", {
        aff: {
          ...form,
          ref_personne: created.id_personne,
          ref_structure: structureId,
        },
      });
      toast.success("Personne et affiliation créées");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  const startQuickAdd = () => {
    setShowQuickAdd(true);
    setForm({ ...form, ref_personne: null });
    setPersonSearch("");
  };

  const cancelQuickAdd = () => {
    setShowQuickAdd(false);
    setNewPerson({
      civilite: null,
      nom: "",
      prenom: "",
      email_prive: null,
      telephone_prive: null,
      consentement_rgpd: false,
    });
    setDuplicatePerson(null);
  };

  return (
    <Modal open={true} onClose={onClose} title={existing ? "Modifier l'affiliation" : "Ajouter une affiliation"}>
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        {isModeStructure && personnes && (
          <div className="md:col-span-2">
            <Label>Personne</Label>
            <input
              value={personSearch}
              onChange={(e) => {
                setPersonSearch(e.target.value);
                setForm({ ...form, ref_personne: null });
              }}
              placeholder="Rechercher par NOM, prénom ou email..."
              disabled={!canSave || showQuickAdd}
              className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
            />
            <div className="mt-1 flex items-center justify-between text-xs text-muted-foreground">
              <span>{personSearchReady ? `${filteredPersonnes.length} proposition(s)` : "Tape au moins 2 caractères pour rechercher."}</span>
              {selectedPersonne && !showQuickAdd && <span>Sélection en cours</span>}
            </div>
            {(personSearchReady || !!selectedPersonne) && (
            <div className="mt-2 max-h-64 overflow-y-auto rounded-xl border bg-background shadow-sm">
              {filteredPersonnes.length > 0 ? (
                filteredPersonnes.map((personne) => {
                  const isSelected = personne.id_personne === form.ref_personne;
                  return (
                    <button
                      key={personne.id_personne}
                      type="button"
                      onClick={() => {
                        setForm({ ...form, ref_personne: personne.id_personne });
                        setPersonSearch(formatPersonOption(personne));
                      }}
                      disabled={!canSave || showQuickAdd}
                      className={`flex w-full items-start justify-between gap-3 border-b border-border/60 px-4 py-3 text-left text-sm transition-colors last:border-b-0 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60 ${isSelected ? "bg-primary/10 text-primary" : "hover:bg-muted/60"}`}
                    >
                      <div className="min-w-0">
                        <div className="truncate font-medium">{formatPersonOption(personne)}</div>
                        <div className="truncate text-xs text-muted-foreground">{personne.email_prive || "Aucun email"}</div>
                      </div>
                      {isSelected && <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[11px] font-semibold">Sélectionné</span>}
                    </button>
                  );
                })
              ) : (
                <div className="px-3 py-4 text-sm text-muted-foreground">Aucune personne ne correspond à la recherche.</div>
              )}
            </div>
            )}
            {selectedPersonne && !showQuickAdd && (
              <p className="mt-2 text-xs text-muted-foreground">
                Sélection actuelle : <span className="font-medium text-foreground">{formatPersonOption(selectedPersonne)}</span>
              </p>
            )}
            {canCreatePerson && !showQuickAdd && (
              <button
                onClick={startQuickAdd}
                className="mt-1.5 text-xs text-primary hover:underline cursor-pointer"
              >
                Nouvelle personne
              </button>
            )}
          </div>
        )}

        {isModePersonne && (
          <div className="md:col-span-2">
            <Label>Structure</Label>
            <input
              value={structureSearch}
              onChange={(e) => {
                setStructureSearch(e.target.value);
                setForm({ ...form, ref_structure: null });
              }}
              placeholder="Rechercher par nom de structure ou commune..."
              disabled={!canSave}
              className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
            />
            <div className="mt-1 flex items-center justify-between text-xs text-muted-foreground">
              <span>{structureSearchReady ? `${filteredStructures.length} proposition(s)` : "Tape au moins 2 caractères pour rechercher."}</span>
              {selectedStructure && <span>Sélection en cours</span>}
            </div>
            {(structureSearchReady || !!selectedStructure) && (
            <div className="mt-2 max-h-64 overflow-y-auto rounded-xl border bg-background shadow-sm">
              {filteredStructures.length > 0 ? (
                filteredStructures.map((structure) => {
                  const isSelected = structure.id_structure === form.ref_structure;
                  return (
                    <button
                      key={structure.id_structure}
                      type="button"
                      onClick={() => {
                        setForm({ ...form, ref_structure: structure.id_structure });
                        setStructureSearch(structure.nom_structure ?? "");
                      }}
                      disabled={!canSave}
                      className={`flex w-full items-start justify-between gap-3 border-b border-border/60 px-4 py-3 text-left text-sm transition-colors last:border-b-0 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60 ${isSelected ? "bg-primary/10 text-primary" : "hover:bg-muted/60"}`}
                    >
                      <div className="min-w-0">
                        <div className="truncate font-medium">{structure.nom_structure || "Structure sans nom"}</div>
                        <div className="truncate text-xs text-muted-foreground">{structure.commune_structure || "Commune non renseignée"}</div>
                      </div>
                      {isSelected && <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[11px] font-semibold">Sélectionnée</span>}
                    </button>
                  );
                })
              ) : (
                <div className="px-3 py-4 text-sm text-muted-foreground">Aucune structure ne correspond à la recherche.</div>
              )}
            </div>
            )}
            {selectedStructure && (
              <p className="mt-2 text-xs text-muted-foreground">
                Sélection actuelle : <span className="font-medium text-foreground">{selectedStructure.nom_structure || "Structure sans nom"}</span>
              </p>
            )}
          </div>
        )}

        {showQuickAdd && (
          <>
            <div className="md:col-span-2 rounded-lg border border-dashed p-3 bg-muted/30">
              <div className="mb-2 flex items-center justify-between">
                <span className="text-xs font-semibold text-muted-foreground">Nouvelle personne</span>
                <button onClick={cancelQuickAdd} className="text-[10px] text-muted-foreground hover:text-foreground cursor-pointer">
                  Annuler
                </button>
              </div>
              <p className="mb-3 text-xs text-muted-foreground">
                La création d’une nouvelle personne remplace la sélection actuelle pour cette affiliation.
              </p>
              <div className="grid grid-cols-2 gap-3 md:grid-cols-3">
                <div>
                  <Label>Civilité</Label>
                  <select
                    value={newPerson.civilite ?? ""}
                    onChange={(e) => setNewPerson({ ...newPerson, civilite: e.target.value || null })}
                    className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm"
                  >
                    <option value="">—</option>
                    <option value="M">M.</option>
                    <option value="Mme">Mme</option>
                    <option value="Mlle">Mlle</option>
                  </select>
                </div>
                <div>
                  <Label>Nom</Label>
                  <input
                    value={newPerson.nom ?? ""}
                    onChange={(e) => setNewPerson({ ...newPerson, nom: normalizeLastNameInput(e.target.value) })}
                    className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm"
                  />
                </div>
                <div>
                  <Label>Prénom</Label>
                  <input
                    value={newPerson.prenom ?? ""}
                    onChange={(e) => setNewPerson({ ...newPerson, prenom: e.target.value })}
                    className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm"
                  />
                </div>
                <div>
                  <Label>Email</Label>
                  <input
                    value={newPerson.email_prive ?? ""}
                    onChange={(e) => setNewPerson({ ...newPerson, email_prive: e.target.value || null })}
                    className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm"
                  />
                </div>
                <div>
                  <Label>Téléphone</Label>
                  <input
                    value={newPerson.telephone_prive ?? ""}
                    onChange={(e) => setNewPerson({ ...newPerson, telephone_prive: e.target.value || null })}
                    className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm"
                  />
                </div>
              </div>
            </div>
          </>
        )}

        <div>
          <Label>Fonction</Label>
          <select
            value={form.ref_fonction ?? ""}
            onChange={(e) => setForm({ ...form, ref_fonction: e.target.value ? Number(e.target.value) : null })}
            disabled={!canSave}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          >
            <option value="">—</option>
            {fonctions.map((f) => (
              <option key={f.id_fonction} value={f.id_fonction}>
                {f.libelle_fonction}
              </option>
            ))}
          </select>
        </div>
        <div>
          <Label>Catégorie</Label>
          <select
            value={form.id_categorie ?? ""}
            onChange={(e) => setForm({ ...form, id_categorie: e.target.value ? Number(e.target.value) : null })}
            disabled={!canSave}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          >
            <option value="">—</option>
            {categories.map((c) => (
              <option key={c.id_categorie} value={c.id_categorie}>
                {c.nom_categorie}
              </option>
            ))}
          </select>
        </div>
        <div>
          <Label>Titre spécifique</Label>
          <input
            value={form.titre_specifique ?? ""}
            onChange={(e) => setForm({ ...form, titre_specifique: e.target.value || null })}
            disabled={!canSave}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          />
        </div>
        <div>
          <Label>Email professionnel</Label>
          <input
            value={form.email_professionnel ?? ""}
            onChange={(e) => setForm({ ...form, email_professionnel: e.target.value || null })}
            disabled={!canSave}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          />
        </div>
        <div>
          <Label>Tél. fixe pro</Label>
          <input
            value={form.telephone_direct ?? ""}
            onChange={(e) => setForm({ ...form, telephone_direct: e.target.value || null })}
            disabled={!canSave}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          />
        </div>
        <div>
          <Label>GSM pro</Label>
          <input
            value={form.gsm_professionnel ?? ""}
            onChange={(e) => setForm({ ...form, gsm_professionnel: e.target.value || null })}
            disabled={!canSave}
            className="mt-1 h-9 w-full rounded-lg border bg-background px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          />
        </div>
      </div>

      {/* Alerte doublon email (quick-add) */}
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
            <button onClick={confirmDuplicateCreate}
              className="cursor-pointer rounded-lg bg-amber-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-amber-700">
              Ajouter quand même
            </button>
          </div>
        </div>
      )}

      {lockBlocked && (
        <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
          Cette affiliation est actuellement éditée par {lockStatus?.holder_label || "un autre utilisateur"}. L’enregistrement est bloqué tant que le verrou est actif.
        </div>
      )}

      <div className="mt-6 flex justify-end gap-3">
        <button onClick={onClose} className="cursor-pointer rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted">
          Annuler
        </button>
        {showQuickAdd ? (
          <button onClick={createPersonAndAffiliation} className="cursor-pointer rounded-xl bg-primary px-6 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90">
            Créer personne + affiliation
          </button>
        ) : (
          canSave && (
            <button disabled={lockBlocked} onClick={save} className="cursor-pointer rounded-xl bg-primary px-6 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60">
              {existing ? "Modifier" : "Ajouter"}
            </button>
          )
        )}
      </div>
    </Modal>
  );
}
