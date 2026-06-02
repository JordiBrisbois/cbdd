import { useState, useEffect, useMemo } from "react";
import type { ReactNode } from "react";
import type { AffiliationAvecDetails, Categorie, Condition, Fonction, Personne, Preset, Reunion, Structure } from "../types";
import { invoke, exportTableFile, formatValuesForMail, splitDelimitedValues } from "../lib/utils";
import { Icon, Label } from "../lib/ui";
import { DataTable, type TableConfig } from "../components/DataTable";
import { TableExportModal, type TableExportFormat, type TableExportScope } from "../components/TableExportModal";
import { CopyValuesModal } from "../components/CopyValuesModal";
import { ContactModal } from "../modals/ContactModal";
import { StructureModal } from "../modals/StructureModal";
import { ReunionModal } from "../modals/ReunionModal";
import { AffiliationModal } from "../modals/AffiliationModal";
import toast from "react-hot-toast";
import { useAuth } from "../lib/auth";

const QB_FIELDS = [
  { alias: "p", table: "Personnes", fields: [
    { key: "p.Nom", label: "Nom" }, { key: "p.Prenom", label: "Prénom" }, { key: "p.Civilite", label: "Civilité" },
    { key: "p.Email_Prive", label: "Email privé" }, { key: "p.Telephone_Prive", label: "Téléphone privé" },
    { key: "p.Adresse_Privee", label: "Adresse" }, { key: "p.Code_Postal_Prive", label: "Code postal" },
    { key: "p.Commune_Privee", label: "Commune" }, { key: "p.Pays", label: "Pays" },
    { key: "p.Statut_Compte", label: "Statut" }, { key: "p.Consentement_RGPD", label: "RGPD" },
    { key: "p.Date_Consentement", label: "Date consentement" }, { key: "p.Notes_Commentaires", label: "Notes" },
    { key: "p.Date_Creation", label: "Date création" },
  ]},
  { alias: "s", table: "Structures", fields: [
    { key: "s.Nom_Structure", label: "Nom" },
    { key: "s.Service_Specifique", label: "Service" }, { key: "s.Reseau_Subvention", label: "Réseau subvention" },
    { key: "s.Partenaire_Direct", label: "Partenaire direct" }, { key: "s.Adresse_Structure", label: "Adresse" },
    { key: "s.Code_Postal_Structure", label: "Code postal" }, { key: "s.Commune_Structure", label: "Commune" },
    { key: "s.Pays", label: "Pays" }, { key: "s.Telephone_General", label: "Téléphone" },
    { key: "s.Email_General", label: "Email" }, { key: "s.Site_Web", label: "Site web" },
    { key: "s.Notes_Commentaires", label: "Notes" },
  ]},
  { alias: "a", table: "Affiliations", fields: [
    { key: "a.Titre_Specifique", label: "Titre spécifique" }, { key: "a.Service_Specifique", label: "Service" },
    { key: "a.Email_Professionnel", label: "Email professionnel" }, { key: "a.Telephone_Direct", label: "Téléphone direct" },
    { key: "a.Gsm_Professionnel", label: "GSM professionnel" }, { key: "a.Date_Debut", label: "Date début" },
    { key: "a.Date_Fin", label: "Date fin" }, { key: "a.Notes_Commentaires", label: "Notes" },
  ]},
  { alias: "c", table: "Catégories", fields: [
    { key: "c.Nom_Categorie", label: "Nom catégorie" },
  ]},
  { alias: "f", table: "Fonctions", fields: [
    { key: "f.Libelle_Fonction", label: "Libellé fonction" },
  ]},
  { alias: "r", table: "Réunions", fields: [
    { key: "r.Titre_Reunion", label: "Titre" }, { key: "r.Date_Reunion", label: "Date" },
    { key: "r.Heure_Reunion", label: "Heure" }, { key: "r.Lieu_Reunion", label: "Lieu" },
    { key: "r.Notes_Commentaires", label: "Notes" },
  ]},
  { alias: "pr", table: "Présences", fields: [
    { key: "pr.Statut_Presence", label: "Statut" }, { key: "pr.Souhaite_Rester_En_BDD", label: "Reste en BDD" },
    { key: "pr.Notes_Commentaires", label: "Notes" },
  ]},
];

const QB_TABLES = [
  { key: "personnes", label: "Personnes", alias: "p" }, { key: "structures", label: "Structures", alias: "s" },
  { key: "affiliations", label: "Affiliations", alias: "a" }, { key: "reunions", label: "Réunions", alias: "r" },
  { key: "presences", label: "Présences", alias: "pr" },
];

const QB_OPERATORS = [
  { key: "=", label: "=" }, { key: "!=", label: "≠" }, { key: ">", label: ">" }, { key: "<", label: "<" },
  { key: ">=", label: "≥" }, { key: "<=", label: "≤" }, { key: "LIKE", label: "contient" },
  { key: "commence_par", label: "commence par" }, { key: "finit_par", label: "finit par" },
  { key: "IS NULL", label: "est vide" }, { key: "IS NOT NULL", label: "n'est pas vide" },
  { key: "is_duplicate", label: "est un doublon" }, { key: "is_not_duplicate", label: "est unique" },
  { key: "in_list", label: "dans la liste" }, { key: "not_in_list", label: "pas dans la liste" },
  { key: "between", label: "entre" },
];

export function SearchPage() {
  const { can } = useAuth();
  const [selectedTable, setSelectedTable] = useState("personnes");
  const [selectedCols, setSelectedCols] = useState<string[]>([]);
  const [conditions, setConditions] = useState<Condition[]>([]);
  const [results, setResults] = useState<Record<string, string>[]>([]);
  const [loading, setLoading] = useState(false);
  const [showColPicker, setShowColPicker] = useState(false);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [presetName, setPresetName] = useState("");
  const [showPresetForm, setShowPresetForm] = useState(false);
  const [showCopyModal, setShowCopyModal] = useState(false);
  const [showExportModal, setShowExportModal] = useState(false);
  const [categories, setCategories] = useState<Categorie[]>([]);
  const [structures, setStructures] = useState<Structure[]>([]);
  const [personnes, setPersonnes] = useState<Personne[]>([]);
  const [fonctions, setFonctions] = useState<Fonction[]>([]);
  const [selectedPersonne, setSelectedPersonne] = useState<Personne | null>(null);
  const [selectedStructure, setSelectedStructure] = useState<Structure | null>(null);
  const [selectedReunion, setSelectedReunion] = useState<Reunion | null>(null);
  const [selectedAffiliation, setSelectedAffiliation] = useState<AffiliationAvecDetails | null>(null);

  const loadPresets = async () => {
    const p = await invoke<Preset[]>("lister_presets").catch(() => []);
    setPresets(p);
  };
  useEffect(() => { loadPresets(); }, []);
  useEffect(() => {
    void invoke<Categorie[]>("lister_categories").then(setCategories).catch(() => setCategories([]));
    void invoke<Structure[]>("lister_structures").then(setStructures).catch(() => setStructures([]));
    void invoke<Personne[]>("lister_personnes").then(setPersonnes).catch(() => setPersonnes([]));
    void invoke<Fonction[]>("lister_fonctions").then(setFonctions).catch(() => setFonctions([]));
  }, []);

  const savePreset = async () => {
    if (!presetName.trim()) { toast.error("Nom du preset requis"); return; }
    await invoke("sauvegarder_preset", {
      preset: { id_preset: null, nom_preset: presetName.trim(), table_principale: selectedTable, colonnes: JSON.stringify(selectedCols), conditions: JSON.stringify(conditions) }
    }).catch((e) => toast.error(String(e)));
    toast.success("Preset enregistré");
    setPresetName("");
    setShowPresetForm(false);
    loadPresets();
  };

  const loadPreset = async (id: number) => {
    const p = await invoke<Preset>("charger_preset", { id }).catch(() => null);
    if (!p) { toast.error("Preset introuvable"); return; }
    setSelectedTable(p.table_principale);
    try { setSelectedCols(JSON.parse(p.colonnes)); } catch { setSelectedCols([]); }
    try { setConditions(JSON.parse(p.conditions)); } catch { setConditions([]); }
    setResults([]);
    toast.success(`Preset "${p.nom_preset}" chargé`);
  };

  const deletePreset = async (id: number) => {
    if (confirm("Supprimer ce preset ?")) {
      try { await invoke("supprimer_preset", { id }); toast.success("Preset supprimé"); } catch (e) { toast.error(String(e)); }
      loadPresets();
    }
  };

  const mainAlias = QB_TABLES.find(t => t.key === selectedTable)?.alias || "p";
  const allFields = QB_FIELDS.flatMap(g => g.fields.map(f => ({ ...f, group: g.table, alias: g.alias })));
  const availableFields = allFields.filter(f => f.alias === mainAlias || ["c", "f", "a", "s", "p", "pr", "r"].includes(f.alias));

  const toggleCol = (key: string) => setSelectedCols(prev => prev.includes(key) ? prev.filter(k => k !== key) : [...prev, key]);
  const addCondition = () => {
    const defaultField = availableFields.find(f => f.alias === mainAlias)?.key || availableFields[0]?.key || "p.Nom";
    setConditions([...conditions, { champ: defaultField, operateur: "LIKE", valeur: "" }]);
  };
  const removeCondition = (i: number) => setConditions(conditions.filter((_, idx) => idx !== i));
  const updateCondition = (i: number, key: keyof Condition, val: string) => {
    const next = [...conditions];
    next[i] = { ...next[i], [key]: val };
    setConditions(next);
  };

  const runQuery = async () => {
    if (selectedCols.length === 0) { toast.error("Sélectionnez au moins une colonne"); return; }
    setLoading(true);
    try {
      const validConditions = conditions.filter(c => {
        const noValueOps = ["IS NULL", "IS NOT NULL", "is_duplicate", "is_not_duplicate"];
        if (noValueOps.includes(c.operateur)) return true;
        return c.valeur?.trim();
      });
      const res = await invoke<Record<string, string>[]>("executer_requete", { tablePrincipale: selectedTable, colonnes: selectedCols, conditions: validConditions });
      setResults(res);
      toast.success(`${res.length} résultat(s)`);
    } catch (e) { toast.error(String(e)); setResults([]); }
    setLoading(false);
  };

  const exportResults = (scope: TableExportScope, format: TableExportFormat) => {
    if (results.length === 0) return;
    const columnsToUse = scope === "current" ? selectedCols : selectedCols;
    const headers = columnsToUse.map(c => { const f = allFields.find(x => x.key === c); return f ? `${f.group} > ${f.label}` : c; });
    const rows = results.map(row => columnsToUse.map(c => row[c.split('.').pop()!] || ""));
    exportTableFile(headers, rows, `query_${selectedTable}_${new Date().toISOString().slice(0, 10)}`, format);
    toast.success(`${results.length} résultats exportés`);
  };

  const selectedTableLabel = QB_TABLES.find(t => t.key === selectedTable)?.label || selectedTable;

  const resultColumns = useMemo(() =>
    selectedCols.map(key => ({
      key: key.split('.').pop()!,
      label: allFields.find(f => f.key === key)?.label || key,
    })),
    [selectedCols],
  );

  const sortAccessors = useMemo(() => {
    const acc: Record<string, (item: Record<string, unknown>) => string> = {};
    resultColumns.forEach(col => { acc[col.key] = (item) => String(item[col.key] ?? ""); });
    return acc;
  }, [resultColumns]);

  const renderers = useMemo(() => {
    const rend: Record<string, (item: Record<string, unknown>) => ReactNode> = {};
    resultColumns.forEach(col => {
      rend[col.key] = (item) => {
        const v = item[col.key];
        return v != null && v !== "" ? String(v) : "—";
      };
    });
    return rend;
  }, [resultColumns]);

  const config: TableConfig = useMemo(() => ({
    id: "query-builder",
    columns: resultColumns,
    sortAccessors,
    pagination: true,
    defaultPageSize: 100,
    emptyText: "Aucun résultat",
  }), [resultColumns, sortAccessors]);

  const copyableFieldOptions = useMemo(() =>
    selectedCols.map((key) => ({
      id: key,
      label: allFields.find((field) => field.key === key)?.label || key,
      description: allFields.find((field) => field.key === key)?.group || undefined,
    })),
  [allFields, selectedCols]);

  const buildFieldCopyPayload = (fieldKey: string) => {
    const columnKey = fieldKey.split(".").pop()!;
    const values = results.flatMap((row) => splitDelimitedValues(row[columnKey]));
    return formatValuesForMail(values);
  };

  const openResult = async (item: Record<string, unknown>) => {
    const entity = String(item._entity ?? "");
    const id = Number(item._id ?? 0);
    if (!entity || !id) return;

    try {
      if (entity === "p" && can("personnes.read")) {
        const personne = await invoke<Personne>("get_personne", { id });
        setSelectedPersonne(personne);
        return;
      }
      if (entity === "s" && can("structures.read")) {
        const structure = await invoke<Structure>("get_structure", { id });
        setSelectedStructure(structure);
        return;
      }
      if (entity === "r" && can("reunions.read")) {
        const reunion = await invoke<Reunion>("get_reunion", { id });
        setSelectedReunion(reunion);
        return;
      }
      if (entity === "a" && can("affiliations.read")) {
        const affiliation = await invoke<AffiliationAvecDetails>("get_affiliation", { id });
        setSelectedAffiliation(affiliation);
        return;
      }
      if (entity === "pr" && can("reunions.read")) {
        const reunionId = Number(item._reunion_id ?? 0);
        if (reunionId) {
          const reunion = await invoke<Reunion>("get_reunion", { id: reunionId });
          setSelectedReunion(reunion);
        }
      }
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">Requête personnalisée</h1>
        <p className="mt-1 text-sm text-muted-foreground">Sélectionnez les colonnes et ajoutez des conditions pour construire votre requête</p>
      </div>

      <div className="rounded-xl border bg-card p-4 shadow-sm">
        <div className="flex items-center justify-between">
          <Label>Colonnes à afficher ({selectedCols.length})</Label>
          <button onClick={() => setShowColPicker(!showColPicker)}
            className="rounded-lg border bg-background px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted cursor-pointer">
            {showColPicker ? "Fermer" : "Sélectionner"}
          </button>
        </div>
        {showColPicker && (
          <div className="mt-3 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {QB_FIELDS.map(group => (
              <div key={group.alias}>
                <span className="text-xs font-semibold text-muted-foreground">{group.table}</span>
                <div className="mt-1 flex flex-col gap-1">
                  {group.fields.map(f => (
                    <label key={f.key} className="flex cursor-pointer items-center gap-2 rounded px-1 py-0.5 text-xs hover:bg-muted/50">
                      <input type="checkbox" checked={selectedCols.includes(f.key)} onChange={() => toggleCol(f.key)}
                        className="rounded border-gray-300 text-primary focus:ring-primary" />
                      <span className="text-muted-foreground">{f.label}</span>
                    </label>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
        {selectedCols.length > 0 && !showColPicker && (
          <div className="mt-2 flex flex-wrap gap-1.5">
            {selectedCols.map(key => {
              const f = allFields.find(x => x.key === key);
              return (
                <span key={key} className="inline-flex items-center gap-1 rounded-md bg-primary/10 px-2 py-1 text-xs font-medium text-primary">
                  {f ? `${f.group} > ${f.label}` : key}
                  <button onClick={() => toggleCol(key)} className="text-primary/70 hover:text-primary cursor-pointer">×</button>
                </span>
              );
            })}
          </div>
        )}
      </div>

      <div className="rounded-xl border bg-card p-4 shadow-sm">
        <div className="flex items-center justify-between">
          <Label>Conditions ({conditions.length})</Label>
          <button onClick={addCondition}
            className="flex items-center gap-1 rounded-lg border border-dashed px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted cursor-pointer">
            <Icon name="plus" className="size-3" /> Ajouter
          </button>
        </div>
        <div className="mt-3 flex flex-col gap-2">
          {conditions.map((cond, i) => (
            <div key={i} className="flex flex-wrap items-end gap-2">
              <select value={cond.champ} onChange={(e) => updateCondition(i, "champ", e.target.value)}
                className="h-9 rounded-lg border bg-background px-3 text-sm">
                {allFields.map(f => <option key={f.key} value={f.key}>{f.group} &gt; {f.label}</option>)}
              </select>
              <select value={cond.operateur} onChange={(e) => updateCondition(i, "operateur", e.target.value)}
                className="h-9 rounded-lg border bg-background px-3 text-sm">
                {QB_OPERATORS.map(op => <option key={op.key} value={op.key}>{op.label}</option>)}
              </select>
              {cond.operateur !== "IS NULL" && cond.operateur !== "IS NOT NULL" && cond.operateur !== "is_duplicate" && cond.operateur !== "is_not_duplicate" && (
                <input value={cond.valeur || ""} onChange={(e) => updateCondition(i, "valeur", e.target.value)}
                  placeholder={cond.operateur === "in_list" || cond.operateur === "not_in_list" ? "val1, val2, val3..." : cond.operateur === "between" || cond.operateur === "entre" ? "début, fin" : "Valeur..."}
                  className="h-9 flex-1 min-w-[150px] rounded-lg border bg-background px-3 text-sm" />
              )}
              <button onClick={() => removeCondition(i)}
                className="flex items-center justify-center h-9 w-9 rounded-lg border bg-background text-red-500 hover:bg-red-50 hover:border-red-200 cursor-pointer" title="Supprimer">
                <Icon name="x" className="size-4" />
              </button>
            </div>
          ))}
          {conditions.length === 0 && <p className="text-xs text-muted-foreground">Aucune condition — tous les enregistrements seront affichés</p>}
        </div>
      </div>

      <div className="rounded-xl border bg-card p-4 shadow-sm">
        <div className="flex items-center justify-between">
          <Label>Presets ({presets.length})</Label>
          <button onClick={() => setShowPresetForm(!showPresetForm)}
            className="rounded-lg border bg-background px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted cursor-pointer">
            {showPresetForm ? "Annuler" : "Enregistrer preset"}
          </button>
        </div>
        {showPresetForm && (
          <div className="mt-2 flex items-center gap-2">
            <input value={presetName} onChange={(e) => setPresetName(e.target.value)} placeholder="Nom du preset..."
              className="h-9 flex-1 min-w-[150px] rounded-lg border bg-background px-3 text-sm" />
            <button onClick={savePreset}
              className="rounded-lg bg-primary px-4 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer">Enregistrer</button>
          </div>
        )}
        {presets.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-1.5">
            {presets.map((p) => (
              <span key={p.id_preset} className="inline-flex items-center gap-1 rounded-md bg-muted px-2 py-1 text-xs">
                <button onClick={() => loadPreset(p.id_preset)} className="hover:text-primary cursor-pointer">{p.nom_preset}</button>
                <button onClick={() => deletePreset(p.id_preset)} className="text-muted-foreground hover:text-red-500 cursor-pointer" title="Supprimer">×</button>
              </span>
            ))}
          </div>
        )}
      </div>

      <div className="flex gap-2">
        <button onClick={runQuery} disabled={loading}
          className="rounded-xl bg-primary px-6 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 disabled:opacity-50 cursor-pointer">
          {loading ? "Exécution..." : "Exécuter"}
        </button>
      </div>

      {results.length > 0 && (
        <div className="flex items-center justify-between">
          <div className="flex flex-col">
            <span className="text-sm text-muted-foreground">{results.length} résultat(s) — {selectedTableLabel}</span>
            <span className="text-xs text-muted-foreground">Cliquez sur une ligne pour ouvrir la fiche correspondante.</span>
          </div>
          <div className="flex gap-2">
            <button onClick={() => setShowCopyModal(true)}
              className="flex items-center gap-1.5 rounded-xl border bg-background px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted transition-colors cursor-pointer">
              <Icon name="mail" className="size-4" /> Copier un champ
            </button>
            <button onClick={() => setShowExportModal(true)}
              className="flex items-center gap-1.5 rounded-xl border bg-background px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted transition-colors cursor-pointer">
              <Icon name="download" className="size-4" /> Exporter
            </button>
          </div>
        </div>
      )}

      {showCopyModal && results.length > 0 && (
        <CopyValuesModal
          title="Copier les valeurs d'un champ"
          description="Choisis la colonne de la requête active à copier. Les valeurs seront dédupliquées puis collées au format séparé par des points-virgules."
          options={copyableFieldOptions}
          defaultOptionId={selectedCols[0]}
          onResolve={buildFieldCopyPayload}
          onClose={() => setShowCopyModal(false)}
        />
      )}
      <TableExportModal
        open={showExportModal}
        onClose={() => setShowExportModal(false)}
        onConfirm={exportResults}
        rawDescription="Exporte les resultats bruts de la requete avec les colonnes selectionnees, sans mise en forme supplementaire."
      />

      {results.length > 0 && (
        <DataTable
          config={config}
          data={results as unknown as Record<string, unknown>[]}
          loading={loading}
          renderers={renderers}
          onRowClick={(item) => { void openResult(item); }}
        />
      )}

      {selectedPersonne && (
        <ContactModal
          personne={selectedPersonne}
          categories={categories}
          onClose={() => setSelectedPersonne(null)}
        />
      )}

      {selectedStructure && (
        <StructureModal
          structure={selectedStructure}
          categories={categories}
          personnes={personnes}
          fonctions={fonctions}
          onClose={() => setSelectedStructure(null)}
        />
      )}

      {selectedReunion && (
        <ReunionModal reunion={selectedReunion} onClose={() => setSelectedReunion(null)} />
      )}

      {selectedAffiliation && (
        <AffiliationModal
          personneId={selectedAffiliation.ref_personne}
          structureId={selectedAffiliation.ref_structure}
          personnes={personnes}
          structures={structures}
          fonctions={fonctions}
          categories={categories}
          existing={selectedAffiliation}
          onClose={() => setSelectedAffiliation(null)}
        />
      )}
    </div>
  );
}
