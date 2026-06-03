import type { ReactNode } from "react";

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
