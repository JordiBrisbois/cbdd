import type { CSSProperties } from "react";
import { cn } from "../../lib/cn";

export function SortHeader({ label, sortKey, currentKey, direction, onToggle, className, style }: {
  label: string; sortKey: string; currentKey: string | null; direction: "asc" | "desc"; onToggle: (key: string) => void;
  className?: string; style?: CSSProperties;
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
