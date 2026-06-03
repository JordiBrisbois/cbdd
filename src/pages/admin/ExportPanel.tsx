import { useState } from "react";
import toast from "react-hot-toast";
import type { ExcelRebuildResult } from "../../types";
import { invoke } from "../../lib/tauri";

export function ExportPanel() {
  const [exportingWorkbook, setExportingWorkbook] = useState(false);

  const exportWorkbook = async () => {
    setExportingWorkbook(true);
    try {
      const result = await invoke<ExcelRebuildResult | null>("exporter_classeur_excel_admin");
      if (!result) {
        toast("Export annulé");
        return;
      }
      toast.success(`${result.row_count} ligne(s) exportée(s) dans ${result.sheet_count} onglets`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExportingWorkbook(false);
    }
  };

  return (
    <div className="rounded-2xl border bg-card p-5 shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold">Export Excel historique</h2>
          <p className="text-sm text-muted-foreground">
            Reconstruit un classeur `.xlsx` à partir de SQLite avec une sheet par catégorie, dans une logique proche de l'ancien fichier.
          </p>
        </div>
        <button
          onClick={() => void exportWorkbook()}
          disabled={exportingWorkbook}
          className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-60"
        >
          {exportingWorkbook ? "Export en cours..." : "Reconstruire le fichier Excel"}
        </button>
      </div>
      <p className="mt-3 text-xs text-muted-foreground">
        Les onglets suivent les catégories métier historiques (`AG`, `CA`, `CPAS`, `CRI`, `ILI`, etc.) et l'export se base d'abord sur la catégorie d'affiliation.
      </p>
    </div>
  );
}
