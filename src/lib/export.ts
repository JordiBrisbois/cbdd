import writeXlsxFile, { type Sheet } from "write-excel-file/browser";

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

export async function exportExcel(headers: string[], rows: string[][], filename: string) {
  await writeXlsxFile([[...headers], ...rows], { sheet: "Export" }).toFile(filename);
}

export async function exportWorkbook(
  sheets: { name: string; headers: string[]; rows: string[][] }[],
  filename: string,
) {
  const workbook: Sheet<Blob>[] = sheets.map((sheet) => ({
    name: sheet.name.slice(0, 31),
    data: [[...sheet.headers], ...sheet.rows],
  }));
  await writeXlsxFile(workbook).toFile(filename);
}

export async function exportTableFile(
  headers: string[],
  rows: string[][],
  filenameBase: string,
  format: "csv" | "excel",
) {
  if (format === "excel") {
    await exportExcel(headers, rows, `${filenameBase}.xlsx`);
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
