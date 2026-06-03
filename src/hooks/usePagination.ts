import { useState, useEffect } from "react";

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
