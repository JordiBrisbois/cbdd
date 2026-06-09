import { useState, useMemo, useCallback } from "react";
import type { Personne, Categorie } from "../types";
import { invoke } from "../lib/tauri";
import { formatCivilite } from "../lib/format";
import { exportTableFile } from "../lib/export";
import { getExportConfig } from "../lib/columns";
import { Icon } from "../lib/ui";
import { DataTable, type TableConfig } from "../components/DataTable";
import { TableExportModal, type TableExportFormat, type TableExportScope } from "../components/TableExportModal";
import { ContactModal } from "../modals/ContactModal";
import toast from "react-hot-toast";
import { useAuth } from "../lib/auth";
import { useAsyncData } from "../hooks/useAsyncData";

export function ContactsPage() {
  const { can } = useAuth();
  const [search, setSearch] = useState("");
  const [catFilter, setCatFilter] = useState<number | null>(null);
  const [selected, setSelected] = useState<Personne | null>(null);
  const [showExportModal, setShowExportModal] = useState(false);
  const canCreatePerson = can("personnes.create");
  const loadPeople = useCallback(() => {
    const categorieId = catFilter && catFilter > 0 ? catFilter : undefined;
    return invoke<Personne[]>("lister_personnes", { recherche: search || undefined, categorieId });
  }, [catFilter, search]);
  const { data: personnes, loading, reload } = useAsyncData(loadPeople, [], {
    errorMessage: "Impossible de charger les contacts",
  });
  const loadCategories = useCallback(() => invoke<Categorie[]>("lister_categories"), []);
  const { data: categories } = useAsyncData(loadCategories, [], {
    errorMessage: "Impossible de charger les catégories",
  });

  const config: TableConfig<Personne> = useMemo(() => ({
    id: "contacts",
    columns: [
      { key: "civ", label: "Civ." }, { key: "nom", label: "Nom" }, { key: "prenom", label: "Prénom" },
      { key: "email", label: "Email privé" }, { key: "tel", label: "Tél." }, { key: "adresse", label: "Adresse" },
      { key: "cp", label: "Code postal" }, { key: "commune", label: "Commune" }, { key: "pays", label: "Pays" },
      { key: "statut", label: "Statut" }, { key: "rgpd", label: "RGPD" }, { key: "notes", label: "Notes" },
      { key: "date_creation", label: "Date création" },
    ],
    sortAccessors: {
      civ: p => p.civilite as string ?? "", nom: p => p.nom as string ?? "", prenom: p => p.prenom as string ?? "",
      email: p => p.email_prive as string ?? "", tel: p => p.telephone_prive as string ?? "",
      adresse: p => p.adresse_privee as string ?? "", cp: p => p.code_postal_prive as string ?? "",
      commune: p => p.commune_privee as string ?? "", pays: p => p.pays as string ?? "",
      statut: p => p.statut_compte as string ?? "", rgpd: p => (p.consentement_rgpd ? "Oui" : "Non") as string,
      notes: p => p.notes_commentaires as string ?? "", date_creation: p => p.date_creation as string ?? "",
    },
    stickyColumns: { widths: { civ: 80, nom: 140, prenom: 132 } },
  }), []);

  const mapContactValue = (p: Personne, k: string) => {
    switch (k) {
      case "civ": return formatCivilite(p.civilite);
      case "nom": return p.nom ?? "";
      case "prenom": return p.prenom ?? "";
      case "email": return p.email_prive ?? "";
      case "tel": return p.telephone_prive ?? "";
      case "adresse": return p.adresse_privee ?? "";
      case "cp": return p.code_postal_prive ?? "";
      case "commune": return p.commune_privee ?? "";
      case "pays": return p.pays ?? "";
      case "statut": return p.statut_compte ?? "";
      case "rgpd": return p.consentement_rgpd ? "Oui" : "Non";
      case "notes": return p.notes_commentaires ?? "";
      case "date_creation": return p.date_creation ?? "";
      default: return "";
    }
  };

  const exportContacts = async (scope: TableExportScope, format: TableExportFormat) => {
    const exportConfig = scope === "current"
      ? getExportConfig("contacts", config.columns)
      : { keysToExport: config.columns.map((column) => column.key), headers: config.columns.map((column) => column.label) };
    const rows = personnes.map(p => exportConfig.keysToExport.map((k: string) => {
      return mapContactValue(p, k);
    }));
    await exportTableFile(exportConfig.headers, rows, `contacts_${new Date().toISOString().slice(0, 10)}`, format);
    toast.success(`${personnes.length} contacts exportés`);
  };

  return (
    <>
      <DataTable config={config} data={personnes} loading={loading}
        renderers={{
          civ: (item) => formatCivilite(item.civilite as string | null),
          nom: (item) => <span className="font-medium">{(item.nom as string) || "—"}</span>,
          prenom: (item) => <span>{(item.prenom as string) || "—"}</span>,
          email: (item) => <span className="text-muted-foreground">{(item.email_prive as string) || "—"}</span>,
          tel: (item) => <span className="text-muted-foreground">{(item.telephone_prive as string) || "—"}</span>,
          adresse: (item) => (item.adresse_privee as string) || "—",
          cp: (item) => (item.code_postal_prive as string) || "—",
          commune: (item) => (item.commune_privee as string) || "—",
          pays: (item) => (item.pays as string) || "—",
          statut: (item) => {
            const s = item.statut_compte as string | null;
            return <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${s === "Actif" ? "bg-emerald-100 text-emerald-800" : s === "Anonymisé" ? "bg-gray-200 text-gray-600" : "bg-amber-100 text-amber-800"}`}>{s || "—"}</span>;
          },
          rgpd: (item) => <span className={`inline-block size-3 rounded-full ${item.consentement_rgpd ? "bg-emerald-500" : "bg-red-400"}`} />,
          notes: (item) => <span className="text-muted-foreground">{(item.notes_commentaires as string) || "—"}</span>,
          date_creation: (item) => <span className="text-muted-foreground">{(item.date_creation as string) || "—"}</span>,
        }}
        onRowClick={setSelected}
        header={
          <div className="flex items-center justify-between">
            <h1 className="text-2xl font-bold tracking-tight">Annuaire des Contacts</h1>
            <div className="flex gap-2">
              <button onClick={() => setShowExportModal(true)} className="flex items-center gap-1.5 rounded-xl border bg-background px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted transition-colors cursor-pointer">
                <Icon name="download" className="size-4" /> Exporter
              </button>
              {canCreatePerson && (
                <button onClick={() => setSelected({
                  id_personne: 0, civilite: null, nom: "", prenom: "", email_prive: null, telephone_prive: null,
                  adresse_privee: null, code_postal_prive: null, commune_privee: null, pays: null,
                  consentement_rgpd: true, date_consentement: null, statut_compte: "Actif", notes_commentaires: null, date_creation: null, updated_at: null
                })}
                  className="flex items-center gap-1.5 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 transition-colors cursor-pointer">
                  <Icon name="plus" className="size-4" /> Nouveau
                </button>
              )}
            </div>
          </div>
        }
        toolbarLeft={
          <div className="flex flex-wrap gap-3">
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Rechercher par nom, prénom, email..."
              className="h-10 min-w-[200px] flex-1 rounded-xl border bg-background px-4 text-sm placeholder:text-muted-foreground focus:border-ring focus:ring-2 focus:ring-ring/30" />
            <select value={catFilter ?? ""} onChange={(e) => setCatFilter(e.target.value ? Number(e.target.value) : null)}
              className="h-10 rounded-xl border bg-background px-3 text-sm focus:border-ring focus:ring-2 focus:ring-ring/30">
              <option value="">Toutes catégories</option>
              {categories.map((c) => <option key={c.id_categorie} value={c.id_categorie}>{c.nom_categorie}</option>)}
            </select>
          </div>
        }
      />
      <TableExportModal open={showExportModal} onClose={() => setShowExportModal(false)} onConfirm={exportContacts} />
      {selected && <ContactModal personne={selected} onClose={() => { setSelected(null); void reload().catch(() => {}); }} categories={categories} />}
    </>
  );
}
