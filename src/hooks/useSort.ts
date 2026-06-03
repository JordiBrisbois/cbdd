import { useState, useMemo } from "react";

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
