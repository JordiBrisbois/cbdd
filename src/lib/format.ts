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
