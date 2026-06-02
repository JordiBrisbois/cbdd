import { useState, useEffect, useCallback } from "react";

export interface ColumnDef {
  key: string;
  label: string;
}

export function useColumnVisibility(tableId: string, allColumns: ColumnDef[]) {
  const [visibleKeys, setVisibleKeys] = useState<Set<string>>(() => {
    const defaultKeys = allColumns.map(c => c.key);
    if (typeof window === "undefined") return new Set(defaultKeys);

    const storageKey = `crvi-columns-${tableId}`;
    const saved = window.localStorage.getItem(storageKey);
    if (!saved) return new Set(defaultKeys);

    try {
      return new Set(JSON.parse(saved));
    } catch {
      return new Set(defaultKeys);
    }
  });

  useEffect(() => {
    const storageKey = `crvi-columns-${tableId}`;
    localStorage.setItem(storageKey, JSON.stringify(Array.from(visibleKeys)));
  }, [tableId, visibleKeys]);

  const toggle = (key: string) => {
    setVisibleKeys(prev => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const setAll = (keys: string[]) => {
    setVisibleKeys(new Set(keys));
  };

  const isVisible = (key: string) => visibleKeys.has(key);

  return { visibleKeys, toggle, setAll, isVisible };
}

export function useStickyColumn(tableId: string) {
  const [stickyKey, setStickyKey] = useState<string | null>(() => {
    if (typeof window === "undefined") return null;
    const saved = window.localStorage.getItem(`crvi-sticky-${tableId}`);
    if (!saved) return null;

    try {
      return JSON.parse(saved);
    } catch {
      return null;
    }
  });

  useEffect(() => {
    localStorage.setItem(`crvi-sticky-${tableId}`, JSON.stringify(stickyKey));
  }, [tableId, stickyKey]);

  const pin = (key: string) => {
    setStickyKey(prev => prev === key ? null : key);
  };

  return { stickyKey, pin };
}

export function useColumnOrder(tableId: string, defaultOrder: string[]) {
  const storageKey = `crvi-order-${tableId}`;

  const [orderedKeys, setOrderedKeys] = useState<string[]>(() => {
    if (typeof window === "undefined") return defaultOrder;
    const saved = localStorage.getItem(storageKey);
    if (!saved) return defaultOrder;
    try {
      const parsed = JSON.parse(saved) as string[];
      const existingSet = new Set(parsed);
      const newCols = defaultOrder.filter(k => !existingSet.has(k));
      return [...parsed.filter(k => existingSet.has(k)), ...newCols];
    } catch {
      return defaultOrder;
    }
  });

  useEffect(() => {
    localStorage.setItem(storageKey, JSON.stringify(orderedKeys));
  }, [tableId, orderedKeys]);

  const moveColumn = useCallback((fromIndex: number, toIndex: number) => {
    setOrderedKeys(prev => {
      const next = [...prev];
      const [moved] = next.splice(fromIndex, 1);
      next.splice(toIndex, 0, moved);
      return next;
    });
  }, []);

  const resetOrder = useCallback(() => {
    setOrderedKeys(defaultOrder);
  }, [defaultOrder]);

  return { orderedKeys, moveColumn, resetOrder };
}

export function useColumnWidths(tableId: string) {
  const storageKey = `crvi-widths-${tableId}`;

  const [widths, setWidths] = useState<Record<string, number>>(() => {
    if (typeof window === "undefined") return {};
    const saved = localStorage.getItem(storageKey);
    if (!saved) return {};
    try {
      const parsed = JSON.parse(saved) as Record<string, number>;
      return Object.fromEntries(
        Object.entries(parsed).filter((entry): entry is [string, number] => Number.isFinite(entry[1]) && entry[1] > 0),
      );
    } catch {
      return {};
    }
  });

  useEffect(() => {
    localStorage.setItem(storageKey, JSON.stringify(widths));
  }, [storageKey, widths]);

  const setWidth = useCallback((key: string, width: number) => {
    const safeWidth = Math.round(width);
    setWidths((prev) => {
      if (!Number.isFinite(safeWidth) || safeWidth <= 0) return prev;
      if (prev[key] === safeWidth) return prev;
      return { ...prev, [key]: safeWidth };
    });
  }, []);

  const resetWidths = useCallback(() => {
    setWidths({});
  }, []);

  return { widths, setWidth, resetWidths };
}

export function getExportConfig(tableId: string, allColumns: ColumnDef[]) {
  const defaultKeys = allColumns.map(c => c.key);
  const savedOrder = typeof window !== "undefined" ? localStorage.getItem(`crvi-order-${tableId}`) : null;
  const savedVisible = typeof window !== "undefined" ? localStorage.getItem(`crvi-columns-${tableId}`) : null;

  const order = savedOrder ? JSON.parse(savedOrder) as string[] : defaultKeys;
  const visible = savedVisible ? new Set(JSON.parse(savedVisible) as string[]) : new Set(defaultKeys);

  const keysToExport = order.filter((k: string) => visible.has(k) && allColumns.some(c => c.key === k));
  const headers = keysToExport.map((k: string) => allColumns.find(c => c.key === k)?.label || k);

  return { keysToExport, headers };
}
