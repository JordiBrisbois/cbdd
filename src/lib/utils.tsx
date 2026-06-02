import type { ReactNode } from "react";
import type { EditLockStatus } from "../types";
import type { ClassValue } from "clsx";
import * as XLSX from "xlsx";
import { clsx } from "clsx";
import { useState, useRef, useEffect, useCallback, useMemo } from "react";

export function cn(...inputs: ClassValue[]) {
  return clsx(inputs);
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (window.__TAURI_INTERNALS__) {
    const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
    return tauriInvoke(cmd, args);
  }
  console.warn(`[CRVI-GRC] Tauri invoke '${cmd}' (not in Tauri context)`);
  throw new Error("Application non lancée depuis Tauri. Utilisez le binaire compilé.");
}

export async function invokeSafe<T>(cmd: string, args?: Record<string, unknown>, fallback?: T): Promise<T | undefined> {
  try { return await invoke<T>(cmd, args); } catch { return fallback; }
}

export function formatDate(d: string | null): string {
  if (!d) return "—";
  try {
    return new Date(d).toLocaleDateString("fr-BE");
  } catch {
    return d;
  }
}

export function formatCivilite(c: string | null): string {
  const civ: Record<string, string> = { M: "M.", Mme: "Mme", Mlle: "Mlle" };
  return c ? civ[c] || c : "";
}

export function fullName(p: { nom: string | null; prenom: string | null }): string {
  return [p.prenom, p.nom].filter(Boolean).join(" ") || "—";
}

export function normalizeLastNameInput(value: string): string {
  return value.toLocaleUpperCase("fr-BE");
}

export function Modal({ open, onClose, title, children }: {
  open: boolean; onClose: () => void; title: string; children: ReactNode;
}) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={onClose}>
      <div className="flex w-[95vw] max-w-6xl max-h-[85vh] flex-col rounded-2xl border bg-card shadow-xl animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}>
        <div className="flex shrink-0 items-center justify-between border-b px-6 py-4">
          <h2 className="text-lg font-semibold">{title}</h2>
          <button onClick={onClose} className="rounded-lg p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer">
            <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
          </button>
        </div>
        <div className="overflow-y-auto px-6 py-4">
          {children}
        </div>
      </div>
    </div>
  );
}

export function Select({ value, onChange, options, placeholder = "Sélectionner...", className = "" }: {
  value: string | number | null; onChange: (v: string) => void;
  options: { value: string | number; label: string }[]; placeholder?: string; className?: string;
}) {
  return (
    <select
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value)}
      className={`h-9 w-full rounded-lg border bg-background px-3 text-sm text-foreground focus:border-ring focus:ring-2 focus:ring-ring/30 ${className}`}
    >
      <option value="" disabled>{placeholder}</option>
      {options.map((o) => (
        <option key={String(o.value)} value={o.value}>{o.label}</option>
      ))}
    </select>
  );
}

export function exportCSV(headers: string[], rows: string[][], filename: string) {
  const bom = "\uFEFF";
  const csv = bom + [headers, ...rows]
    .map((r) => r.map((c) => `"${(c ?? "").replace(/"/g, '""')}"`).join(";"))
    .join("\r\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export function exportExcel(headers: string[], rows: string[][], filename: string) {
  const worksheet = XLSX.utils.aoa_to_sheet([headers, ...rows]);
  const workbook = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(workbook, worksheet, "Export");
  XLSX.writeFile(workbook, filename, { bookType: "xlsx", compression: true });
}

export function exportWorkbook(
  sheets: { name: string; headers: string[]; rows: string[][] }[],
  filename: string,
) {
  const workbook = XLSX.utils.book_new();
  sheets.forEach((sheet) => {
    const worksheet = XLSX.utils.aoa_to_sheet([sheet.headers, ...sheet.rows]);
    XLSX.utils.book_append_sheet(workbook, worksheet, sheet.name.slice(0, 31));
  });
  XLSX.writeFile(workbook, filename, { bookType: "xlsx", compression: true });
}

export function exportTableFile(
  headers: string[],
  rows: string[][],
  filenameBase: string,
  format: "csv" | "excel",
) {
  if (format === "excel") {
    exportExcel(headers, rows, `${filenameBase}.xlsx`);
    return;
  }
  exportCSV(headers, rows, `${filenameBase}.csv`);
}

export function exportCSVVisible(headers: string[], rows: string[][], visibleKeys: Set<string>, columnDefs: { key: string; idx: number }[]) {
  const toExport = columnDefs.filter(c => visibleKeys.has(c.key));
  const filteredHeaders = toExport.map(c => headers[c.idx]);
  const filteredRows = rows.map(row => toExport.map(c => row[c.idx]));
  exportCSV(filteredHeaders, filteredRows, `export_${Date.now()}.csv`);
}

export function splitDelimitedValues(value: string | null | undefined): string[] {
  if (!value) return [];
  return value
    .split(/[;,\n]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function uniqueValues(values: Array<string | null | undefined>): string[] {
  return Array.from(new Set(values.map((value) => value?.trim().toLowerCase()).filter(Boolean) as string[]));
}

export function formatValuesForMail(values: Array<string | null | undefined>): string {
  return uniqueValues(values).join("; ");
}

/**
 * Extrait le texte brut d'un ReactNode (string, number, élément, fragment...)
 */
export function extractText(node: ReactNode): string {
  if (node == null || typeof node === "boolean") return "";
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(extractText).join("");
  if ("props" in (node as object)) {
    const props = (node as { props?: { children?: ReactNode } }).props;
    return props?.children ? extractText(props.children) : "";
  }
  return "";
}

export async function copyText(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const input = document.createElement("textarea");
  input.value = text;
  input.setAttribute("readonly", "true");
  input.style.position = "fixed";
  input.style.opacity = "0";
  document.body.appendChild(input);
  input.select();
  document.execCommand("copy");
  document.body.removeChild(input);
}

export function usePagination<T>(items: T[], defaultSize = 100) {
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(defaultSize);

  const totalPages = Math.max(1, Math.ceil(items.length / pageSize));
  const start = (page - 1) * pageSize;
  const paginated = items.slice(start, start + pageSize);

  // Reset to page 1 when items or pageSize change
  useEffect(() => {
    setPage(1);
  }, [items.length, pageSize]);

  const goTo = (p: number) => setPage(Math.max(1, Math.min(p, totalPages)));

  return { page, pageSize, setPageSize, totalPages, paginated, goTo, start, total: items.length };
}

export function useEditLock(resourceType: string, resourceId: number | null | undefined, enabled: boolean) {
  const [lockStatus, setLockStatus] = useState<EditLockStatus | null>(null);
  const [lockLoading, setLockLoading] = useState(false);

  useEffect(() => {
    if (!enabled || !resourceId) {
      setLockStatus(null);
      setLockLoading(false);
      return;
    }

    let active = true;
    setLockLoading(true);

    void invoke<EditLockStatus>("acquire_edit_lock", { resourceType, resourceId })
      .then((status) => {
        if (active) setLockStatus(status);
      })
      .catch(() => {
        if (active) setLockStatus(null);
      })
      .finally(() => {
        if (active) setLockLoading(false);
      });

    return () => {
      active = false;
      void invoke("release_edit_lock", { resourceType, resourceId }).catch(() => {});
    };
  }, [enabled, resourceId, resourceType]);

  return {
    lockStatus,
    lockLoading,
    lockBlocked: !!lockStatus && !lockStatus.acquired,
  };
}

export function GroupedCell({ value, max = 2 }: { value: string | null; max?: number }) {
  if (!value) return <span className="text-muted-foreground">—</span>;
  const items = value.split(",").map(s => s.trim()).filter(Boolean);
  if (items.length === 0) return <span className="text-muted-foreground">—</span>;
  const visible = items.slice(0, max);
  const remaining = items.length - max;
  const title = items.join("\n");
  return (
    <span title={title} className="inline-flex flex-wrap items-center gap-1 cursor-help">
      {visible.map((item, i) => (
        <span key={i} className="inline-flex items-center rounded-md bg-muted px-1.5 py-0.5 text-xs">{item}</span>
      ))}
      {remaining > 0 && (
        <span className="inline-flex items-center rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">+{remaining}</span>
      )}
    </span>
  );
}

export function ColumnToggle({ tableId, columns, visibleKeys, onToggle, onSelectAll, onDeselectAll, stickyKey, onPin, orderedKeys, onMoveColumn, onResetOrder, onResetWidths }: {
  tableId: string; columns: { key: string; label: string }[]; visibleKeys?: Set<string>;
  onToggle?: (key: string) => void; onSelectAll?: () => void; onDeselectAll?: () => void;
  stickyKey?: string | null; onPin?: (key: string) => void;
  orderedKeys?: string[]; onMoveColumn?: (fromIndex: number, toIndex: number) => void; onResetOrder?: () => void;
  onResetWidths?: () => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const controlled = visibleKeys != null && onToggle != null;
  const state = controlled ? visibleKeys : undefined;
  const [internalSelected, setInternalSelected] = useState<Set<string>>(() => {
    const saved = localStorage.getItem(`crvi-columns-${tableId}`);
    if (saved) {
      try { return new Set(JSON.parse(saved)); } catch { return new Set(columns.map(c => c.key)); }
    }
    return new Set(columns.map(c => c.key));
  });
  const selected = state ?? internalSelected;

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  useEffect(() => {
    if (!controlled) {
      localStorage.setItem(`crvi-columns-${tableId}`, JSON.stringify(Array.from(selected)));
    }
  }, [controlled, tableId, selected]);

  const toggle = (key: string) => {
    if (controlled) {
      onToggle(key);
    } else {
      setInternalSelected(prev => {
        const next = new Set(prev);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        return next;
      });
    }
  };

  const selectAll = () => {
    if (controlled && onSelectAll) {
      onSelectAll();
    } else if (controlled && onToggle) {
      columns.filter(c => !visibleKeys!.has(c.key)).forEach(c => onToggle(c.key));
    } else {
      setInternalSelected(new Set(columns.map(c => c.key)));
    }
  };
  const deselectAll = () => {
    if (controlled && onDeselectAll) {
      onDeselectAll();
    } else if (controlled && onToggle) {
      Array.from(visibleKeys!).forEach(key => onToggle(key));
    } else {
      setInternalSelected(new Set());
    }
  };

  return (
    <div ref={ref} className="relative">
      <button onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 rounded-lg border bg-background px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors cursor-pointer"
        title="Afficher/masquer les colonnes">
        <svg className="size-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
        </svg>
        Colonnes
        <svg className={`size-3 transition-transform ${open ? "rotate-180" : ""}`} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19.5 8.25l-7.5 7.5-7.5-7.5" /></svg>
      </button>
      {open && (
        <div className="absolute left-0 z-[100] mt-1 w-64 rounded-xl border bg-card p-3 shadow-xl">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-xs font-semibold text-muted-foreground">Colonnes visibles</span>
            <div className="flex items-center gap-1">
              <button onClick={selectAll} className="cursor-pointer text-[10px] text-primary hover:underline">Tout</button>
              <span className="text-muted-foreground">/</span>
              <button onClick={deselectAll} className="cursor-pointer text-[10px] text-muted-foreground hover:underline">Aucun</button>
            </div>
          </div>
          {(onResetOrder || onResetWidths) && (
            <div className="mb-2 grid gap-1">
              {onResetOrder && (
                <button onClick={onResetOrder} className="flex w-full items-center justify-center gap-1 rounded-lg border border-dashed py-1.5 text-[11px] text-muted-foreground hover:bg-muted hover:text-foreground transition-colors cursor-pointer">
                  <svg className="size-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
                  Réinitialiser l'ordre et les largeurs
                </button>
              )}
              {onResetWidths && (
                <button onClick={onResetWidths} className="flex w-full items-center justify-center gap-1 rounded-lg border border-dashed py-1.5 text-[11px] text-muted-foreground hover:bg-muted hover:text-foreground transition-colors cursor-pointer">
                  <svg className="size-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7h16M7 4v16m10-16v16M4 17h16" /></svg>
                  Réinitialiser les largeurs
                </button>
              )}
            </div>
          )}
          <div className="max-h-72 overflow-y-auto">
            {(orderedKeys || columns.map(c => c.key)).map((key, index, arr) => {
              const col = columns.find(c => c.key === key);
              if (!col) return null;
              const isFirst = index === 0;
              const isLast = index === arr.length - 1;
              const sk = stickyKey;
              const stickyIdx = sk ? arr.indexOf(sk) : -1;
              const canMove = !sk || index > stickyIdx;
              const canMoveUp = canMove && !isFirst;
              const canMoveDown = canMove && !isLast;
              return (
                <label key={col.key} className="flex cursor-pointer items-center gap-1 py-1.5 text-xs hover:bg-muted/50 rounded px-1 group">
                  <button onClick={(e) => { e.preventDefault(); if (onMoveColumn && canMoveUp) onMoveColumn(index, index - 1); }}
                    disabled={!canMoveUp}
                    className="p-0.5 disabled:opacity-20 hover:bg-muted rounded cursor-pointer disabled:cursor-not-allowed"
                    title="Déplacer à gauche">◀</button>
                  <button onClick={(e) => { e.preventDefault(); if (onMoveColumn && canMoveDown) onMoveColumn(index, index + 1); }}
                    disabled={!canMoveDown}
                    className="p-0.5 disabled:opacity-20 hover:bg-muted rounded cursor-pointer disabled:cursor-not-allowed"
                    title="Déplacer à droite">▶</button>
                  <input type="checkbox" checked={selected.has(col.key)} onChange={() => toggle(col.key)}
                    className="rounded border-gray-300 text-primary focus:ring-primary" />
                  <span className="flex-1 text-muted-foreground">{col.label}</span>
                  {col.key === stickyKey && <span className="text-[10px] text-primary font-medium" title="Figée">📌</span>}
                  {onPin && (
                    <button onClick={(e) => { e.stopPropagation(); onPin(col.key); }}
                      className={`p-0.5 rounded cursor-pointer ${stickyKey === col.key ? 'text-primary bg-primary/10' : 'text-muted-foreground/20 hover:text-muted-foreground'}`}
                      title={stickyKey === col.key ? "Défiger" : "Figer jusqu'à cette colonne"}>
                      <svg className="size-3" fill="currentColor" viewBox="0 0 24 24"><path d="m16 12 6-12H2l6 12v6l-2 4h12l-2-4z"/></svg>
                    </button>
                  )}
                </label>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

export function useTableScroll(containerRef: React.RefObject<HTMLDivElement | null>) {
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);

  const update = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    setCanScrollLeft(el.scrollLeft > 2);
    setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 2);
  }, [containerRef]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    update();
    el.addEventListener("scroll", update);
    window.addEventListener("resize", update);
    const observer = new MutationObserver(update);
    observer.observe(el, { childList: true, subtree: true, attributes: true });
    return () => {
      el.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
      observer.disconnect();
    };
  }, [containerRef, update]);

  const scrollLeft = () => containerRef.current?.scrollBy({ left: -300, behavior: "smooth" });
  const scrollRight = () => containerRef.current?.scrollBy({ left: 300, behavior: "smooth" });

  return { canScrollLeft, canScrollRight, scrollLeft, scrollRight };
}

export function ScrollIndicators({ canScrollLeft, canScrollRight, scrollLeft, scrollRight }: {
  canScrollLeft: boolean; canScrollRight: boolean; scrollLeft: () => void; scrollRight: () => void;
}) {
  return (
    <div className="flex items-center gap-1">
      <button onClick={canScrollLeft ? scrollLeft : undefined} disabled={!canScrollLeft}
        className="flex items-center justify-center size-7 rounded-lg border bg-background hover:bg-muted cursor-pointer text-muted-foreground disabled:opacity-20 disabled:cursor-default"
        title="Défiler vers la gauche">&#8249;</button>
      <button onClick={canScrollRight ? scrollRight : undefined} disabled={!canScrollRight}
        className="flex items-center justify-center size-7 rounded-lg border bg-background hover:bg-muted cursor-pointer text-muted-foreground disabled:opacity-20 disabled:cursor-default"
        title="Défiler vers la droite">&#8250;</button>
    </div>
  );
}

export function ScrollGradients({ canScrollLeft, canScrollRight }: {
  canScrollLeft: boolean; canScrollRight: boolean;
}) {
  return (
    <>
      {canScrollLeft && <div className="pointer-events-none absolute left-0 top-0 bottom-0 w-10 rounded-l-xl bg-gradient-to-r from-card to-transparent z-[1]" />}
      {canScrollRight && <div className="pointer-events-none absolute right-0 top-0 bottom-0 w-10 rounded-r-xl bg-gradient-to-l from-card to-transparent z-[1]" />}
    </>
  );
}

export function FloatingScrollIndicators({ canScrollLeft, canScrollRight, scrollLeft, scrollRight }: {
  canScrollLeft: boolean; canScrollRight: boolean; scrollLeft: () => void; scrollRight: () => void;
}) {
  return (
    <>
      {canScrollLeft && (
        <button
          onClick={scrollLeft}
          className="absolute left-2 top-1/2 -translate-y-1/2 size-6 flex items-center justify-center rounded-full bg-background/50 backdrop-blur-sm border cursor-pointer hover:bg-muted transition-colors z-20"
          title="Défiler vers la gauche">‹</button>
      )}
      {canScrollRight && (
        <button
          onClick={scrollRight}
          className="absolute right-2 top-1/2 -translate-y-1/2 size-6 flex items-center justify-center rounded-full bg-background/50 backdrop-blur-sm border cursor-pointer hover:bg-muted transition-colors z-20"
          title="Défiler vers la droite">›</button>
      )}
    </>
  );
}

export function FloatingScrollBar({ canScrollLeft, canScrollRight, scrollLeft, scrollRight }: {
  canScrollLeft: boolean; canScrollRight: boolean; scrollLeft: () => void; scrollRight: () => void;
}) {
  if (!canScrollLeft && !canScrollRight) return null;
  return (
    <div className="fixed bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-4 rounded-full bg-background/70 backdrop-blur-sm border shadow-sm px-6 py-2.5 z-50">
      <button onClick={canScrollLeft ? scrollLeft : undefined} disabled={!canScrollLeft}
        className="flex items-center justify-center size-9 rounded-full hover:bg-muted transition-colors cursor-pointer disabled:opacity-20 disabled:cursor-default text-foreground/70 text-lg" title="Défiler vers la gauche">‹</button>
      <span className="text-xs text-muted-foreground select-none">Défiler</span>
      <button onClick={canScrollRight ? scrollRight : undefined} disabled={!canScrollRight}
        className="flex items-center justify-center size-9 rounded-full hover:bg-muted transition-colors cursor-pointer disabled:opacity-20 disabled:cursor-default text-foreground/70 text-lg" title="Défiler vers la droite">›</button>
    </div>
  );
}

export const STICKY_COL_WIDTHS: Record<string, number> = {
  civ: 80, nom: 140, prenom: 132, email: 180, tel: 120,
  adresse: 180, cp: 100, commune: 140, pays: 100, statut: 100,
  rgpd: 70, notes: 120, date_creation: 120,
  structure: 160, fonction: 140, categorie: 120, emailPro: 180,
  titre: 140, telDirect: 120, gsmPro: 120, type: 120,
  service: 140, reseau: 120, partenaire: 140, site: 120,
  consentement: 120, date: 120, heure: 80, lieu: 140,
  organisme: 140, reunion: 140,
};

export function useStickyOffsets(
  visibleKeys: Set<string>,
  stickyKey: string | null,
  columnOrder: string[],
  widths?: Record<string, number>,
) {
  const offsets: Record<string, number> = {};
  let left = 0;
  let lastStickyKey = "";
  if (!stickyKey) return { offsets, lastStickyKey };
  for (const col of columnOrder) {
    if (visibleKeys.has(col)) {
      offsets[col] = left;
      lastStickyKey = col;
      left += widths?.[col] || STICKY_COL_WIDTHS[col] || 120;
    }
    if (col === stickyKey) break;
  }
  return { offsets, lastStickyKey };
}

export function stickyCol(left: number, isLast = false, bg?: string, isHeader = false): React.CSSProperties {
  const zBase = isHeader ? 40 : 30;
  return {
    position: "sticky",
    left,
    zIndex: isLast ? zBase + 1 : zBase,
    backgroundColor: bg,
    backgroundImage: "none",
    opacity: 1,
    backgroundClip: "padding-box",
    borderRight: isLast ? "1px solid hsl(var(--border) / 0.3)" : undefined,
    boxShadow: isLast ? "2px 0 0 hsl(var(--border) / 0.22), 10px 0 18px -14px rgb(15 23 42 / 0.35)" : undefined,
  };
}

export function useSort<T>(items: T[], accessors: Record<string, (item: T) => string>) {
  const [sortKey, setSortKey] = useState<string | null>(null);
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");

  const toggle = (key: string) => {
    if (sortKey === key) {
      setSortDir(d => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
  };

  const sorted = useMemo(() => {
    if (!sortKey || !accessors[sortKey]) return items;
    const accessor = accessors[sortKey];
    return [...items].sort((a, b) => {
      const aVal = accessor(a).toLowerCase();
      const bVal = accessor(b).toLowerCase();
      const cmp = aVal.localeCompare(bVal, "fr", { numeric: true });
      return sortDir === "asc" ? cmp : -cmp;
    });
  }, [items, sortKey, sortDir, accessors]);

  return { sorted, sortKey, sortDir, toggle };
}

export function SortHeader({ label, sortKey, currentKey, direction, onToggle, className, style }: {
  label: string; sortKey: string; currentKey: string | null; direction: "asc" | "desc"; onToggle: (key: string) => void;
  className?: string; style?: React.CSSProperties;
}) {
  const active = currentKey === sortKey;
  return (
    <div className={cn("px-4 py-3 font-medium text-muted-foreground cursor-pointer select-none hover:text-foreground transition-colors whitespace-nowrap", className)} style={style} onClick={() => onToggle(sortKey)}>
      <span className="inline-flex items-center gap-1">
        {label}
        {active && (
          <svg className="size-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            {direction === "asc" ? (
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4.5 15.75l7.5-7.5 7.5 7.5" />
            ) : (
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
            )}
          </svg>
        )}
        {!active && <svg className="size-3.5 text-muted-foreground/30" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.25 15L12 18.75 15.75 15m-7.5-6L12 5.25 15.75 9" /></svg>}
      </span>
    </div>
  );
}

