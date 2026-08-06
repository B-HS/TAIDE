import { read, utils } from 'xlsx'
import type { WorkBook, WorkSheet } from 'xlsx'

export type SpreadsheetCellValue = string | number | boolean | null

export type SpreadsheetRow = SpreadsheetCellValue[]

export type SpreadsheetSheet = {
    name: string
    rows: SpreadsheetRow[]
    totalRowCount: number
    truncated: boolean
}

export const SPREADSHEET_PREVIEW_MAX_ROWS = 500

const sheetToRows = (worksheet: WorkSheet) => utils.sheet_to_json<SpreadsheetRow>(worksheet, { header: 1, raw: true, defval: null })

const buildSheetPreview = (name: string, worksheet: WorkSheet | undefined): SpreadsheetSheet => {
    const allRows = worksheet ? sheetToRows(worksheet) : []
    const truncated = allRows.length > SPREADSHEET_PREVIEW_MAX_ROWS

    return {
        name,
        rows: truncated ? allRows.slice(0, SPREADSHEET_PREVIEW_MAX_ROWS) : allRows,
        totalRowCount: allRows.length,
        truncated,
    }
}

export const workbookToSheets = (workbook: WorkBook): SpreadsheetSheet[] =>
    workbook.SheetNames.map((name) => buildSheetPreview(name, workbook.Sheets[name]))

export const parseSpreadsheetWorkbook = (data: ArrayBuffer): WorkBook => read(data, { type: 'array' })
