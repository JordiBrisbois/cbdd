import { useState, useEffect, useMemo } from "react";
import type { Structure, Categorie, Personne, Fonction } from "../types";
import { invoke, exportTableFile } from "../lib/utils";
import { getExportConfig } from "../lib/columns";
import { Icon } from "../lib/ui";
import { DataTable, type TableConfig } from "../components/DataTable";
import { TableExportModal, type TableExportFormat, type TableExportScope } from "../components/TableExportModal";
import { StructureModal } from "../modals/StructureModal";
import toast from "react-hot-toast";
import { useAuth } from "../lib/auth";

export function StructuresPage() {
  const { can } = useAuth();
  const [items, setItems] = useState<Structure[]>([]);
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Structure | null>(null);
  const [showExportModal, setShowExportModal] = useState(false);
  const [loading, setLoading] = useState(false);
  const [categories, setCategories] = useState<Categorie[]>([]);
  const [catMap, setCatMap] = useState<Record<number, string>>({});
  const [personnes, setPersonnes] = useState<Personne[]>([]);
  const [fonctions, setFonctions] = useState<Fonction[]>([]);
  const canCreateStructure = can("structures.create");

  useEffect(() => {
    void invoke<Categorie[]>("lister_categories")
      .then(cats => {
        setCategories(cats);
        const m: Record<number, string> = {};
        cats.forEach(c => { if (c.id_categorie) m[c.id_categorie] = c.nom_categorie ?? ""; });
        setCatMap(m);
      })
      .catch(() => setCategories([]));
    void invoke<Personne[]>("lister_personnes").then(setPersonnes).catch((e) => toast.error(String(e)));
    void invoke<Fonction[]>("lister_fonctions").then(setFonctions).catch((e) => toast.error(String(e)));
  }, []);

  const load = async () => {
    setLoading(true);
    const s = await invoke<Structure[]>("lister_structures", { recherche: search || undefined }).catch(() => []);
    setItems(s);
    setLoading(false);
  };
  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { load(); }, [search]);

  const config: TableConfig = useMemo(() => ({
    id: "structures",
    columns: [
      { key: "nom", label: "Nom" }, { key: "categorie", label: "Catégorie" }, { key: "service", label: "Service" },
      { key: "reseau", label: "Réseau" }, { key: "partenaire", label: "Partenaire direct" },
      { key: "adresse", label: "Adresse" }, { key: "cp", label: "Code postal" }, { key: "commune", label: "Commune" },
      { key: "pays", label: "Pays" }, { key: "tel", label: "Tél." }, { key: "email", label: "Email" },
      { key: "site", label: "Site web" },
    ],
    sortAccessors: {
      nom: s => s.nom as string ?? "", categorie: s => s.categorie as string ?? "", service: s => s.service as string ?? "",
      reseau: s => s.reseau as string ?? "", partenaire: s => s.partenaire ? "Oui" : "Non",
      adresse: s => s.adresse as string ?? "", cp: s => s.cp as string ?? "", commune: s => s.commune as string ?? "",
      pays: s => s.pays as string ?? "", tel: s => s.tel as string ?? "", email: s => s.email as string ?? "",
      site: s => s.site as string ?? "",
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

  const exportStructures = (scope: TableExportScope, format: TableExportFormat) => {
    const exportConfig = scope === "current"
      ? getExportConfig("structures", config.columns)
      : { keysToExport: config.columns.map((column) => column.key), headers: config.columns.map((column) => column.label) };
    const rows = items.map(s => exportConfig.keysToExport.map((k: string) => {
      return mapStructureValue(s, k);
    }));
    exportTableFile(exportConfig.headers, rows, `structures_${new Date().toISOString().slice(0, 10)}`, format);
    toast.success(`${items.length} structures exportées`);
  };

  return (
    <>
      <DataTable config={config} data={items as unknown as Record<string, unknown>[]} loading={loading}
        renderers={{
          nom: (item) => <span className="font-medium">{(item.nom_structure as string) || "—"}</span>,
          categorie: (item) => {
            const catId = (item as any).id_categorie as number | undefined;
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
        onRowClick={(item) => setSelected(item as unknown as Structure)}
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
      {selected && <StructureModal structure={selected} onClose={() => { setSelected(null); load(); }} categories={categories} personnes={personnes} fonctions={fonctions} />}
    </>
  );
}
