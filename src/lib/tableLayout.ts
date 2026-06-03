import type { CSSProperties } from "react";

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

export function stickyCol(left: number, isLast = false, bg?: string, isHeader = false): CSSProperties {
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
