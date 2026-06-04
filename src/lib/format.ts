export function formatDate(d: string | null): string {
  if (!d) return "—";

  const legacyDate = d.match(/^(\d{2})\/(\d{2})\/(\d{2}|\d{4})$/);
  if (legacyDate) {
    const [, day, month, rawYear] = legacyDate;
    const year = rawYear.length === 2 ? `20${rawYear}` : rawYear;
    return `${day}/${month}/${year}`;
  }

  const parsed = new Date(d);
  if (Number.isNaN(parsed.getTime())) return d;

  return parsed.toLocaleDateString("fr-BE");
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
