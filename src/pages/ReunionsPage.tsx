import { useState, useMemo, useCallback } from "react";
import type { Reunion } from "../types";
import { invoke } from "../lib/tauri";
import { formatDate } from "../lib/format";
import { exportTableFile } from "../lib/export";
import { getExportConfig } from "../lib/columns";
import { Icon } from "../lib/ui";
import { DataTable, type TableConfig } from "../components/DataTable";
import { TableExportModal, type TableExportFormat, type TableExportScope } from "../components/TableExportModal";
import { ReunionModal } from "../modals/ReunionModal";
import toast from "react-hot-toast";
import { useAuth } from "../lib/auth";
import { useAsyncData } from "../hooks/useAsyncData";

export function ReunionsPage() {
  const { can } = useAuth();
  const [selected, setSelected] = useState<Reunion | null>(null);
  const [search, setSearch] = useState("");
  const [showExportModal, setShowExportModal] = useState(false);
  const canCreateReunion = can("reunions.create");
  const loadReunions = useCallback(
    () => invoke<Reunion[]>("lister_reunions", { recherche: search || undefined }),
    [search],
  );
  const { data: items, loading, reload } = useAsyncData(loadReunions, [], {
    errorMessage: "Impossible de charger les réunions",
  });

  const config: TableConfig<Reunion> = useMemo(() => ({
    id: "reunions",
    columns: [
      { key: "titre", label: "Titre" }, { key: "date", label: "Date" }, { key: "heure", label: "Heure" },
      { key: "lieu", label: "Lieu" }, { key: "organisme", label: "Organisme" }, { key: "notes", label: "Notes" },
    ],
    sortAccessors: {
      titre: r => r.titre_reunion ?? "", date: r => r.date_reunion ?? "",
      heure: r => r.heure_reunion ?? "", lieu: r => r.lieu_reunion ?? "",
      organisme: r => r.nom_structure ?? "", notes: r => r.notes_commentaires ?? "",
    },
    stickyColumns: { widths: { titre: 160, date: 120, lieu: 140 } },
  }), []);

  const mapReunionValue = (reunion: Reunion, key: string) => {
    switch (key) {
      case "titre": return reunion.titre_reunion ?? "";
      case "date": return formatDate(reunion.date_reunion);
      case "heure": return reunion.heure_reunion ?? "";
      case "lieu": return reunion.lieu_reunion ?? "";
      case "organisme": return reunion.nom_structure ?? "";
      case "notes": return reunion.notes_commentaires ?? "";
      default: return "";
    }
  };

  const exportReunions = async (scope: TableExportScope, format: TableExportFormat) => {
    const exportConfig = scope === "current"
      ? getExportConfig("reunions", config.columns)
      : { keysToExport: config.columns.map((column) => column.key), headers: config.columns.map((column) => column.label) };
    const rows = items.map((reunion) => exportConfig.keysToExport.map((key) => mapReunionValue(reunion, key)));
    await exportTableFile(exportConfig.headers, rows, `reunions_${new Date().toISOString().slice(0, 10)}`, format);
    toast.success(`${items.length} réunions exportées`);
  };

  return (
    <>
      <DataTable config={config} data={items} loading={loading}
        renderers={{
          titre: (item) => <span className="font-medium">{(item.titre_reunion as string) || "—"}</span>,
          date: (item) => formatDate(item.date_reunion as string | null),
          heure: (item) => (item.heure_reunion as string) || "—",
          lieu: (item) => <span className="text-muted-foreground">{(item.lieu_reunion as string) || "—"}</span>,
          organisme: (item) => <span className="text-muted-foreground">{(item.nom_structure as string) || "—"}</span>,
          notes: (item) => <span className="text-muted-foreground">{(item.notes_commentaires as string) || "—"}</span>,
        }}
        onRowClick={setSelected}
        header={
          <div className="flex items-center justify-between">
            <h1 className="text-2xl font-bold tracking-tight">Suivi des Réunions</h1>
            <div className="flex gap-2">
              <button onClick={() => setShowExportModal(true)} className="flex items-center gap-1.5 rounded-xl border bg-background px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted transition-colors cursor-pointer">
                <Icon name="download" className="size-4" /> Exporter
              </button>
              {canCreateReunion && (
                <button onClick={() => setSelected({
                  id_reunion: 0, titre_reunion: "", date_reunion: "", heure_reunion: "",
                  lieu_reunion: null, ref_structure: null, nom_structure: null, notes_commentaires: null, updated_at: null
                })}
                  className="flex items-center gap-1.5 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 cursor-pointer">
                  <Icon name="plus" className="size-4" /> Nouvelle
                </button>
              )}
            </div>
          </div>
        }
        toolbarLeft={
          <div className="flex flex-1">
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Rechercher par titre, date, lieu, organisme..."
              className="h-10 min-w-[200px] flex-1 rounded-xl border bg-background px-4 text-sm placeholder:text-muted-foreground focus:border-ring focus:ring-2 focus:ring-ring/30" />
          </div>
        }
      />
      <TableExportModal open={showExportModal} onClose={() => setShowExportModal(false)} onConfirm={exportReunions} />
      {selected && <ReunionModal reunion={selected} onClose={() => { setSelected(null); void reload().catch(() => {}); }} />}
    </>
  );
}
