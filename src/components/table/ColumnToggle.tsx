import { useState, useRef, useEffect } from "react";

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
