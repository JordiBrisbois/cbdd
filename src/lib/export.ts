import * as XLSX from "xlsx";

export function exportCSV(headers: string[], rows: string[][], filename: string) {
  const bom = "\uFEFF";
  const csv = bom + [headers, ...rows]
    .map((r) => r.map((c) => `"${(c ?? "").replace(/"/g, '""')}"`).join(";"))
    .join("\r\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export function exportExcel(headers: string[], rows: string[][], filename: string) {
  const worksheet = XLSX.utils.aoa_to_sheet([headers, ...rows]);
  const workbook = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(workbook, worksheet, "Export");
  XLSX.writeFile(workbook, filename, { bookType: "xlsx", compression: true });
}

export function exportWorkbook(
  sheets: { name: string; headers: string[]; rows: string[][] }[],
  filename: string,
) {
  const workbook = XLSX.utils.book_new();
  sheets.forEach((sheet) => {
    const worksheet = XLSX.utils.aoa_to_sheet([sheet.headers, ...sheet.rows]);
    XLSX.utils.book_append_sheet(workbook, worksheet, sheet.name.slice(0, 31));
  });
  XLSX.writeFile(workbook, filename, { bookType: "xlsx", compression: true });
}

export function exportTableFile(
  headers: string[],
  rows: string[][],
  filenameBase: string,
  format: "csv" | "excel",
) {
  if (format === "excel") {
    exportExcel(headers, rows, `${filenameBase}.xlsx`);
    return;
  }
  exportCSV(headers, rows, `${filenameBase}.csv`);
}

export function exportCSVVisible(headers: string[], rows: string[][], visibleKeys: Set<string>, columnDefs: { key: string; idx: number }[]) {
  const toExport = columnDefs.filter(c => visibleKeys.has(c.key));
  const filteredHeaders = toExport.map(c => headers[c.idx]);
  const filteredRows = rows.map(row => toExport.map(c => row[c.idx]));
  exportCSV(filteredHeaders, filteredRows, `export_${Date.now()}.csv`);
}
