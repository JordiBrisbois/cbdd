import type { ReactNode } from "react";
import { useRef, useMemo, useCallback } from "react";
import { DragDropContext, Droppable, Draggable, type DropResult } from "@hello-pangea/dnd";
import { useColumnVisibility, useStickyColumn, useColumnOrder, useColumnWidths } from "../lib/columns";
import { ColumnToggle } from "./table/ColumnToggle";
import { ScrollIndicators, ScrollGradients, FloatingScrollBar, FloatingScrollIndicators } from "./table/ScrollControls";
import { SortHeader } from "./table/SortHeader";
import { usePagination } from "../hooks/usePagination";
import { useTableScroll } from "../hooks/useTableScroll";
import { useSort } from "../hooks/useSort";
import { useStickyOffsets, stickyCol } from "../lib/tableLayout";
import { extractText } from "../lib/text";

export interface TableConfig {
  id: string;
  columns: { key: string; label: string }[];
  sortAccessors: Record<string, (item: Record<string, unknown>) => string>;
  stickyColumns?: { widths: Record<string, number> };
  pagination?: boolean;
  pageSizes?: number[];
  defaultPageSize?: number;
  emptyText?: string;
}

export interface DataTableProps {
  config: TableConfig;
  data: Record<string, unknown>[];
  renderers: Record<string, (item: Record<string, unknown>) => ReactNode>;
  onRowClick?: (item: Record<string, unknown>) => void;
  header?: ReactNode;
  toolbarLeft?: ReactNode;
  rowClassName?: string;
  loading?: boolean;
}

export function DataTable({ config, data, renderers, onRowClick, header, toolbarLeft, rowClassName, loading }: DataTableProps) {
  const colVisibility = useColumnVisibility(config.id, config.columns);
  const sticky = useStickyColumn(config.id);
  const columnWidths = useColumnWidths(config.id);
  const stickyKey = config.stickyColumns ? sticky.stickyKey : null;
  const pin = config.stickyColumns ? sticky.pin : undefined;
  const enableOrder = config.id !== "query-builder";
  const { orderedKeys: baseOrderedKeys, moveColumn, resetOrder } = useColumnOrder(
    config.id,
    config.columns.map(c => c.key),
  );

  // Appliquer l'ordre + sticky en premier (freeze panes: bloc 0..stickyKey figé)
  const columnOrder = useMemo(() => {
    if (!enableOrder) return config.columns.map(c => c.key);
    const order = baseOrderedKeys.filter(k => config.columns.some(c => c.key === k));
    if (stickyKey && order.includes(stickyKey)) {
      const stickyIdx = order.indexOf(stickyKey);
      return [...order.slice(0, stickyIdx + 1), ...order.slice(stickyIdx + 1)];
    }
    return order;
  }, [baseOrderedKeys, config.columns, stickyKey, enableOrder]);

  const { offsets, lastStickyKey } = useStickyOffsets(
    colVisibility.visibleKeys,
    stickyKey,
    columnOrder,
    { ...config.stickyColumns?.widths, ...columnWidths.widths },
  );
  const sortAccessors = useMemo(() => config.sortAccessors, [config.sortAccessors]);
  const { sorted, sortKey, sortDir, toggle: toggleSort } = useSort(data, sortAccessors);
  const paginationEnabled = config.pagination !== false;
  const sizes = config.pageSizes || [50, 100, 250, 500, 1000];
  const { page, pageSize, setPageSize, totalPages, paginated, goTo, start, total } = usePagination(
    sorted,
    paginationEnabled ? config.defaultPageSize || 100 : data.length,
  );
  const tableRef = useRef<HTMLDivElement>(null);
  const headerRefs = useRef<Record<string, HTMLTableCellElement | null>>({});
  const { canScrollLeft, canScrollRight, scrollLeft, scrollRight } = useTableScroll(tableRef);

  const visibleCols = columnOrder.filter(c => colVisibility.isVisible(c));
  const effectiveWidths = useMemo(
    () => ({ ...config.stickyColumns?.widths, ...columnWidths.widths }),
    [config.stickyColumns?.widths, columnWidths.widths],
  );

  // DnD handler
  const onDragEnd = useCallback((result: DropResult) => {
    if (!result.destination || !enableOrder) return;
    const fromIdx = result.source.index;
    const toIdx = result.destination.index;
    if (fromIdx === toIdx) return;

    // Récupérer les clés visibles (l'ordre du DnD)
    const visOrder = visibleCols;
    const draggedKey = visOrder[fromIdx];
    const targetKey = visOrder[toIdx];

    // Vérifier la barrière sticky dans l'ordre complet (non visible)
    const fullOrder = columnOrder;
    const stickyActualIdx = stickyKey ? fullOrder.indexOf(stickyKey) : -1;
    const draggedActualIdx = fullOrder.indexOf(draggedKey);
    const targetActualIdx = fullOrder.indexOf(targetKey);

    // Aucune colonne du bloc figé ne peut être déplacée
    if (stickyKey && stickyActualIdx >= 0 && draggedActualIdx <= stickyActualIdx) return;

    // Interdire de traverser la barrière sticky
    if (stickyKey && stickyActualIdx >= 0) {
      const draggedBeforeSticky = draggedActualIdx < stickyActualIdx;
      const targetAfterSticky = targetActualIdx >= stickyActualIdx;
      const draggedAfterSticky = draggedActualIdx > stickyActualIdx;
      const targetBeforeSticky = targetActualIdx <= stickyActualIdx;

      if ((draggedBeforeSticky && targetAfterSticky) || (draggedAfterSticky && targetBeforeSticky)) {
        return; // Traversée du sticky interdite
      }
    }

    // Convertir en indices dans l'ordre réel (baseOrderedKeys)
    const realFrom = baseOrderedKeys.indexOf(draggedKey);
    const realTo = baseOrderedKeys.indexOf(targetKey);
    if (realFrom >= 0 && realTo >= 0 && realFrom !== realTo) {
      moveColumn(realFrom, realTo);
    }
  }, [visibleCols, columnOrder, baseOrderedKeys, stickyKey, enableOrder, moveColumn]);

  const getStickyStyle = (colKey: string, isHeader: boolean) => {
    const offset = offsets[colKey];
    if (offset === undefined) return undefined;

    const width = effectiveWidths[colKey];
    const isLast = lastStickyKey === colKey;

    return {
      ...stickyCol(
        offset,
        isLast,
        isHeader ? "var(--color-muted)" : "var(--color-card)",
        isHeader,
      ),
      width,
      minWidth: width,
      maxWidth: width,
    };
  };

  const handleResizeStart = useCallback((colKey: string, event: React.PointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.stopPropagation();

    const header = headerRefs.current[colKey];
    if (!header) return;

    const startX = event.clientX;
    const startWidth = header.getBoundingClientRect().width;
    const minWidth = 96;
    const maxWidth = 640;
    const previousCursor = document.body.style.cursor;
    const previousSelect = document.body.style.userSelect;

    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const handleMove = (moveEvent: PointerEvent) => {
      const nextWidth = Math.min(maxWidth, Math.max(minWidth, startWidth + moveEvent.clientX - startX));
      columnWidths.setWidth(colKey, nextWidth);
    };

    const handleUp = () => {
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousSelect;
      window.removeEventListener("pointermove", handleMove);
    };

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp, { once: true });
  }, [columnWidths]);

  const renderHeader = (colKey: string, index: number): ReactNode => {
    const col = config.columns.find(c => c.key === colKey);
    const label = col?.label || colKey;
    const stickyStyle = getStickyStyle(colKey, true);

    const stickyIdx = stickyKey ? columnOrder.indexOf(stickyKey) : -1;
    const colIdx = columnOrder.indexOf(colKey);
    const isDragDisabled = !enableOrder || (stickyKey !== null && colIdx <= stickyIdx);

    const headerContent = <SortHeader label={label} sortKey={colKey} currentKey={sortKey} direction={sortDir} onToggle={toggleSort} />;
    const resizeHandle = (
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label={`Redimensionner la colonne ${label}`}
        onPointerDown={(event) => handleResizeStart(colKey, event)}
        className="absolute inset-y-0 right-0 z-[120] w-3 translate-x-1/2 cursor-col-resize touch-none"
      >
        <div className="absolute right-1.5 top-1/2 h-7 w-px -translate-y-1/2 rounded-full bg-border transition-colors group-hover:bg-primary/40" />
      </div>
    );

    if (!enableOrder) {
      return (
        <th
          key={colKey}
          ref={(node) => {
            headerRefs.current[colKey] = node;
          }}
          className="group relative"
          style={stickyStyle || undefined}
        >
          {headerContent}
          {resizeHandle}
        </th>
      );
    }

    return (
      <Draggable key={colKey} draggableId={colKey} index={index} isDragDisabled={isDragDisabled}>
        {(provided, snapshot) => (
          <th
            ref={(node) => {
              provided.innerRef(node);
              headerRefs.current[colKey] = node;
            }}
            {...provided.draggableProps}
            {...provided.dragHandleProps}
            className="group relative"
            style={{
              ...provided.draggableProps.style,
              ...(stickyStyle || {}),
              ...(snapshot.isDragging ? { opacity: 0.85, zIndex: 100 } : {}),
            }}
          >
            {headerContent}
            {resizeHandle}
          </th>
        )}
      </Draggable>
    );
  };

  const renderCell = (colKey: string, item: Record<string, unknown>) => {
    const stickyStyle = getStickyStyle(colKey, false);
    const cell = renderers[colKey]?.(item) ?? <span className="text-muted-foreground">—</span>;
    const text = extractText(cell);

    if (stickyStyle) {
      return (
        <td key={colKey}
            className="px-4 py-2.5 [background-color:var(--color-card)] group-hover:[background-color:var(--color-muted)] transition-colors"
            style={stickyStyle}>
          <div className="truncate" title={text || undefined}>{cell}</div>
        </td>
      );
    }
    return (
      <td key={colKey} className="px-4 py-2.5">
        <div className="max-w-[40ch] truncate" title={text || undefined}>{cell}</div>
      </td>
    );
  };

  const getId = (item: Record<string, unknown>, index: number): string | number =>
    (item.id_personne ?? item.id_structure ?? item.id_reunion ?? item.id_categorie ?? `row-${index}`) as string | number;

  return (
    <div className="flex flex-col gap-4">
      {header}

      {toolbarLeft && <div>{toolbarLeft}</div>}

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <ColumnToggle tableId={config.id} columns={config.columns}
            visibleKeys={colVisibility.visibleKeys} onToggle={colVisibility.toggle}
            onSelectAll={() => colVisibility.setAll(config.columns.map(c => c.key))}
            onDeselectAll={() => colVisibility.setAll([])}
            stickyKey={config.stickyColumns ? stickyKey : undefined}
            onPin={pin}
            orderedKeys={enableOrder ? columnOrder : undefined}
            onMoveColumn={enableOrder ? (fromIdx, toIdx) => {
              const fromKey = columnOrder[fromIdx];
              const toKey = columnOrder[toIdx];
              const realFrom = baseOrderedKeys.indexOf(fromKey);
              const realTo = baseOrderedKeys.indexOf(toKey);
              if (realFrom >= 0 && realTo >= 0) moveColumn(realFrom, realTo);
            } : undefined}
            onResetOrder={enableOrder ? () => {
              resetOrder();
              columnWidths.resetWidths();
            } : undefined}
            onResetWidths={enableOrder ? undefined : columnWidths.resetWidths} />
          <ScrollIndicators canScrollLeft={canScrollLeft} canScrollRight={canScrollRight} scrollLeft={scrollLeft} scrollRight={scrollRight} />
        </div>
        {paginationEnabled && (
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">Afficher</span>
            <select value={pageSize} onChange={(e) => setPageSize(Number(e.target.value))}
              className="h-8 rounded-lg border bg-background px-2 text-xs">
              {sizes.map(s => <option key={s} value={s}>{s}</option>)}
            </select>
          </div>
        )}
      </div>

      <div className="relative isolate">
        <div ref={tableRef} className="overflow-x-auto rounded-xl border bg-card shadow-sm">
          <table className="min-w-full w-max text-sm border-separate border-spacing-0">
            <colgroup>
              {visibleCols.map((colKey) => {
                const width = effectiveWidths[colKey];
                return <col key={colKey} style={width ? { width, minWidth: width } : undefined} />;
              })}
            </colgroup>
            <thead className="sticky top-0 z-10">
              {enableOrder ? (
                <DragDropContext onDragEnd={onDragEnd}>
                  <Droppable droppableId={`${config.id}-headers`} direction="horizontal">
                    {(provided) => (
                      <tr ref={provided.innerRef} {...provided.droppableProps} className="border-b bg-muted/50 text-left">
                        {visibleCols.map((colKey, index) => renderHeader(colKey, index))}
                        {provided.placeholder}
                      </tr>
                    )}
                  </Droppable>
                </DragDropContext>
              ) : (
                <tr className="border-b bg-muted/50 text-left">
                  {visibleCols.map((colKey, index) => renderHeader(colKey, index))}
                </tr>
              )}
            </thead>
            <tbody>
              {(paginationEnabled ? paginated : data).map((item, index) => (
                <tr key={getId(item, index)} onClick={() => onRowClick?.(item)}
                  className={`group cursor-pointer border-b last:border-0 hover:bg-muted/50 transition-colors${rowClassName ? ` ${rowClassName}` : ""}`}>
                  {visibleCols.map(colKey => renderCell(colKey, item))}
                </tr>
              ))}
              {loading && (
                <tr><td colSpan={visibleCols.length} className="px-4 py-12 text-center">
                  <svg className="size-5 animate-spin mx-auto text-muted-foreground" fill="none" viewBox="0 0 24 24"><circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"/><path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
                  <span className="block mt-2 text-xs text-muted-foreground">Chargement...</span>
                </td></tr>
              )}
              {!loading && visibleCols.length > 0 && data.length === 0 && (
                <tr><td colSpan={visibleCols.length} className="px-4 py-8 text-center text-muted-foreground">{config.emptyText || "Aucune donnée"}</td></tr>
              )}
            </tbody>
          </table>
        </div>
        <ScrollGradients canScrollLeft={canScrollLeft} canScrollRight={canScrollRight} />
        <FloatingScrollIndicators canScrollLeft={canScrollLeft} canScrollRight={canScrollRight} scrollLeft={scrollLeft} scrollRight={scrollRight} />
        <FloatingScrollBar canScrollLeft={canScrollLeft} canScrollRight={canScrollRight} scrollLeft={scrollLeft} scrollRight={scrollRight} />
      </div>

      {paginationEnabled && total > 0 && (
        <div className="flex items-center justify-between rounded-xl border bg-card px-4 py-2.5 shadow-sm">
          <span className="text-xs text-muted-foreground">
            {start + 1}–{Math.min(start + pageSize, total)} sur {total}
          </span>
          <div className="flex items-center gap-1">
            <button onClick={() => goTo(page - 1)} disabled={page <= 1}
              className="h-8 w-8 rounded-lg border bg-background text-sm hover:bg-muted disabled:opacity-40 cursor-pointer">‹</button>
            <span className="min-w-[3rem] text-center text-sm font-medium">{page} / {totalPages}</span>
            <button onClick={() => goTo(page + 1)} disabled={page >= totalPages}
              className="h-8 w-8 rounded-lg border bg-background text-sm hover:bg-muted disabled:opacity-40 cursor-pointer">›</button>
          </div>
        </div>
      )}
    </div>
  );
}
