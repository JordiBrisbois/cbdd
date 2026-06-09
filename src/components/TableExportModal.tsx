import { useState } from "react";
import toast from "react-hot-toast";
import { Modal } from "../components/Modal";
import { Icon } from "../lib/ui";

export type TableExportScope = "current" | "raw";
export type TableExportFormat = "csv" | "excel";

export function TableExportModal({
  open,
  onClose,
  onConfirm,
  currentLabel = "Exporter la vue actuelle",
  currentDescription = "Utilise les colonnes visibles, leur ordre actuel et les donnees affichees.",
  rawLabel = "Exporter le brut complet",
  rawDescription = "Utilise toutes les colonnes prevues pour cette table, sans se limiter a la vue courante.",
}: {
  open: boolean;
  onClose: () => void;
  onConfirm: (scope: TableExportScope, format: TableExportFormat) => Promise<void> | void;
  currentLabel?: string;
  currentDescription?: string;
  rawLabel?: string;
  rawDescription?: string;
}) {
  const [scope, setScope] = useState<TableExportScope>("current");
  const [format, setFormat] = useState<TableExportFormat>("csv");
  const [exporting, setExporting] = useState(false);

  const confirmExport = async () => {
    setExporting(true);
    try {
      await onConfirm(scope, format);
      onClose();
    } catch (error) {
      toast.error(`Impossible d'exporter les données: ${String(error)}`);
    } finally {
      setExporting(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="Exporter les donnees">
      <div className="flex flex-col gap-5">
        <div>
          <h3 className="text-sm font-semibold">Portee</h3>
          <div className="mt-3 grid gap-3">
            <label className={`cursor-pointer rounded-xl border p-3 transition-colors ${scope === "current" ? "border-primary bg-primary/5" : "hover:bg-muted/50"}`}>
              <div className="flex items-start gap-3">
                <input
                  type="radio"
                  name="export-scope"
                  checked={scope === "current"}
                  onChange={() => setScope("current")}
                  className="mt-1"
                />
                <div>
                  <div className="text-sm font-medium">{currentLabel}</div>
                  <p className="mt-1 text-xs text-muted-foreground">{currentDescription}</p>
                </div>
              </div>
            </label>
            <label className={`cursor-pointer rounded-xl border p-3 transition-colors ${scope === "raw" ? "border-primary bg-primary/5" : "hover:bg-muted/50"}`}>
              <div className="flex items-start gap-3">
                <input
                  type="radio"
                  name="export-scope"
                  checked={scope === "raw"}
                  onChange={() => setScope("raw")}
                  className="mt-1"
                />
                <div>
                  <div className="text-sm font-medium">{rawLabel}</div>
                  <p className="mt-1 text-xs text-muted-foreground">{rawDescription}</p>
                </div>
              </div>
            </label>
          </div>
        </div>

        <div>
          <h3 className="text-sm font-semibold">Format</h3>
          <div className="mt-3 flex gap-3">
            <button
              type="button"
              onClick={() => setFormat("csv")}
              className={`rounded-xl border px-4 py-2 text-sm font-medium transition-colors cursor-pointer ${format === "csv" ? "border-primary bg-primary/5 text-primary" : "hover:bg-muted"}`}
            >
              CSV
            </button>
            <button
              type="button"
              onClick={() => setFormat("excel")}
              className={`rounded-xl border px-4 py-2 text-sm font-medium transition-colors cursor-pointer ${format === "excel" ? "border-primary bg-primary/5 text-primary" : "hover:bg-muted"}`}
            >
              Excel
            </button>
          </div>
        </div>

        <div className="flex justify-end gap-3">
          <button onClick={onClose} className="cursor-pointer rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted">
            Annuler
          </button>
          <button
            onClick={() => void confirmExport()}
            disabled={exporting}
            className="flex cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
          >
            <Icon name="download" className="size-4" /> {exporting ? "Export en cours..." : "Exporter"}
          </button>
        </div>
      </div>
    </Modal>
  );
}
