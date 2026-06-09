import type { ReactNode } from "react";

export function Modal({ open, onClose, title, children, closable = true }: {
  open: boolean; onClose: () => void; title: string; children: ReactNode; closable?: boolean;
}) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={closable ? onClose : undefined}>
      <div className="flex w-[95vw] max-w-6xl max-h-[85vh] flex-col rounded-2xl border bg-card shadow-xl animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}>
        <div className="flex shrink-0 items-center justify-between border-b px-6 py-4">
          <h2 className="text-lg font-semibold">{title}</h2>
          {closable && (
            <button onClick={onClose} className="rounded-lg p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer">
              <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
            </button>
          )}
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
