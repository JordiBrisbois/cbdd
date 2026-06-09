import { useState, useMemo, useCallback } from "react";
import type { Structure, Categorie, Personne, Fonction } from "../types";
import { invoke } from "../lib/tauri";
import { exportTableFile } from "../lib/export";
import { getExportConfig } from "../lib/columns";
import { Icon } from "../lib/ui";
import { DataTable, type TableConfig } from "../components/DataTable";
import { TableExportModal, type TableExportFormat, type TableExportScope } from "../components/TableExportModal";
import { StructureModal } from "../modals/StructureModal";
import toast from "react-hot-toast";
import { useAuth } from "../lib/auth";
import { useAsyncData } from "../hooks/useAsyncData";

type StructureReferences = {
  categories: Categorie[];
  personnes: Personne[];
  fonctions: Fonction[];
};

export function StructuresPage() {
  const { can } = useAuth();
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Structure | null>(null);
  const [showExportModal, setShowExportModal] = useState(false);
  const canCreateStructure = can("structures.create");
  const loadStructures = useCallback(
    () => invoke<Structure[]>("lister_structures", { recherche: search || undefined }),
    [search],
  );
  const { data: items, loading, reload } = useAsyncData(loadStructures, [], {
    errorMessage: "Impossible de charger les structures",
  });
  const loadReferences = useCallback(async (): Promise<StructureReferences> => {
    const [categories, personnes, fonctions] = await Promise.allSettled([
      invoke<Categorie[]>("lister_categories"),
      invoke<Personne[]>("lister_personnes"),
      invoke<Fonction[]>("lister_fonctions"),
    ]);
    return {
      categories: categories.status === "fulfilled" ? categories.value : [],
      personnes: personnes.status === "fulfilled" ? personnes.value : [],
      fonctions: fonctions.status === "fulfilled" ? fonctions.value : [],
    };
  }, []);
  const { data: references } = useAsyncData<StructureReferences>(
    loadReferences,
    { categories: [], personnes: [], fonctions: [] },
    { errorMessage: "Impossible de charger les données de référence" },
  );
  const { categories, personnes, fonctions } = references;
  const catMap = useMemo(() => Object.fromEntries(
    categories
      .filter((category) => category.id_categorie != null)
      .map((category) => [category.id_categorie as number, category.nom_categorie ?? ""]),
  ), [categories]);

  const config: TableConfig<Structure> = useMemo(() => ({
    id: "structures",
    columns: [
      { key: "nom", label: "Nom" }, { key: "categorie", label: "Catégorie" }, { key: "service", label: "Service" },
      { key: "reseau", label: "Réseau" }, { key: "partenaire", label: "Partenaire direct" },
      { key: "adresse", label: "Adresse" }, { key: "cp", label: "Code postal" }, { key: "commune", label: "Commune" },
      { key: "pays", label: "Pays" }, { key: "tel", label: "Tél." }, { key: "email", label: "Email" },
      { key: "site", label: "Site web" },
    ],
    sortAccessors: {
      nom: s => s.nom_structure ?? "", categorie: s => String(s.id_categorie ?? ""), service: s => s.service_specifique ?? "",
      reseau: s => s.reseau_subvention ?? "", partenaire: s => s.partenaire_direct ? "Oui" : "Non",
      adresse: s => s.adresse_structure ?? "", cp: s => s.code_postal_structure ?? "", commune: s => s.commune_structure ?? "",
      pays: s => s.pays ?? "", tel: s => s.telephone_general ?? "", email: s => s.email_general ?? "",
      site: s => s.site_web ?? "",
    },
    stickyColumns: { widths: { nom: 140, categorie: 140 } },
  }), []);

  const mapStructureValue = (s: Structure, k: string) => {
    switch (k) {
      case "nom": return s.nom_structure ?? "";
      case "categorie": return catMap[s.id_categorie ?? 0] ?? "";
      case "service": return s.service_specifique ?? "";
      case "reseau": return s.reseau_subvention ?? "";
      case "partenaire": return s.partenaire_direct ? "Oui" : "Non";
      case "adresse": return s.adresse_structure ?? "";
      case "cp": return s.code_postal_structure ?? "";
      case "commune": return s.commune_structure ?? "";
      case "pays": return s.pays ?? "";
      case "tel": return s.telephone_general ?? "";
      case "email": return s.email_general ?? "";
      case "site": return s.site_web ?? "";
      default: return "";
    }
  };

  const exportStructures = async (scope: TableExportScope, format: TableExportFormat) => {
    const exportConfig = scope === "current"
      ? getExportConfig("structures", config.columns)
      : { keysToExport: config.columns.map((column) => column.key), headers: config.columns.map((column) => column.label) };
    const rows = items.map(s => exportConfig.keysToExport.map((k: string) => {
      return mapStructureValue(s, k);
    }));
    await exportTableFile(exportConfig.headers, rows, `structures_${new Date().toISOString().slice(0, 10)}`, format);
    toast.success(`${items.length} structures exportées`);
  };

  return (
    <>
      <DataTable config={config} data={items} loading={loading}
        renderers={{
          nom: (item) => <span className="font-medium">{(item.nom_structure as string) || "—"}</span>,
          categorie: (item) => {
            const catId = item.id_categorie;
            return <span className="text-muted-foreground">{catId ? (catMap[catId] || "—") : "—"}</span>;
          },
          service: (item) => <span className="text-muted-foreground">{(item.service_specifique as string) || "—"}</span>,
          reseau: (item) => <span className="text-muted-foreground">{(item.reseau_subvention as string) || "—"}</span>,
          partenaire: (item) => item.partenaire_direct ? "Oui" : "Non",
          adresse: (item) => (item.adresse_structure as string) || "—",
          cp: (item) => (item.code_postal_structure as string) || "—",
          commune: (item) => (item.commune_structure as string) || "—",
          pays: (item) => (item.pays as string) || "—",
          tel: (item) => <span className="text-muted-foreground">{(item.telephone_general as string) || "—"}</span>,
          email: (item) => <span className="text-muted-foreground">{(item.email_general as string) || "—"}</span>,
          site: (item) => <span className="text-muted-foreground">{(item.site_web as string) || "—"}</span>,
        }}
        onRowClick={setSelected}
        header={
          <div className="flex items-center justify-between">
            <h1 className="text-2xl font-bold tracking-tight">Répertoire des Structures</h1>
            <div className="flex gap-2">
              <button onClick={() => setShowExportModal(true)} className="flex items-center gap-1.5 rounded-xl border bg-background px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted transition-colors cursor-pointer">
                <Icon name="download" className="size-4" /> Exporter
              </button>
              {canCreateStructure && (
                <button onClick={() => setSelected({
                  id_structure: 0, nom_structure: "", service_specifique: null, reseau_subvention: null,
                  partenaire_direct: false, adresse_structure: null, code_postal_structure: null, commune_structure: null,
                  pays: null, telephone_general: null, email_general: null, site_web: null, notes_commentaires: null, date_creation: null, id_categorie: null, updated_at: null
                })}
                  className="flex items-center gap-1.5 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 cursor-pointer">
                  <Icon name="plus" className="size-4" /> Nouveau
                </button>
              )}
            </div>
          </div>
        }
        toolbarLeft={
          <div className="flex flex-1">
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Rechercher par nom, commune..."
              className="h-10 min-w-[200px] flex-1 rounded-xl border bg-background px-4 text-sm placeholder:text-muted-foreground focus:border-ring focus:ring-2 focus:ring-ring/30" />
          </div>
        }
      />
      <TableExportModal open={showExportModal} onClose={() => setShowExportModal(false)} onConfirm={exportStructures} />
      {selected && <StructureModal structure={selected} onClose={() => { setSelected(null); void reload().catch(() => {}); }} categories={categories} personnes={personnes} fonctions={fonctions} />}
    </>
  );
}
