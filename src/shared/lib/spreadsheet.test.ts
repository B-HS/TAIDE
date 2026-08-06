import { describe, expect, test } from 'bun:test'
import { utils } from 'xlsx'
import type { WorkBook } from 'xlsx'
import { SPREADSHEET_PREVIEW_MAX_ROWS, workbookToSheets } from '@shared/lib/spreadsheet'

const buildWorkbook = (sheets: { name: string; rows: unknown[][] }[]): WorkBook => {
    const workbook = utils.book_new()
    sheets.forEach(({ name, rows }) => utils.book_append_sheet(workbook, utils.aoa_to_sheet(rows), name))
    return workbook
}

describe('workbookToSheets', () => {
    test('빈 시트는 빈 rows 를 반환한다', () => {
        const workbook = buildWorkbook([{ name: 'Empty', rows: [] }])

        const sheets = workbookToSheets(workbook)

        expect(sheets).toEqual([{ name: 'Empty', rows: [], totalRowCount: 0, truncated: false }])
    })

    test('병합 없는 단순 표를 행 배열로 변환한다', () => {
        const workbook = buildWorkbook([
            {
                name: 'Sheet1',
                rows: [
                    ['name', 'age'],
                    ['kim', 30],
                ],
            },
        ])

        const sheets = workbookToSheets(workbook)

        expect(sheets).toEqual([
            {
                name: 'Sheet1',
                rows: [
                    ['name', 'age'],
                    ['kim', 30],
                ],
                totalRowCount: 2,
                truncated: false,
            },
        ])
    })

    test('숫자와 문자가 섞인 셀의 타입을 보존한다', () => {
        const workbook = buildWorkbook([
            {
                name: 'Mixed',
                rows: [
                    ['label', 'value', 'active'],
                    ['a', 1, true],
                    ['b', 2.5, false],
                ],
            },
        ])

        const [sheet] = workbookToSheets(workbook)

        expect(sheet.rows[1]).toEqual(['a', 1, true])
        expect(typeof sheet.rows[1][1]).toBe('number')
        expect(sheet.rows[2]).toEqual(['b', 2.5, false])
    })

    test('여러 시트를 워크북의 순서대로 변환한다', () => {
        const workbook = buildWorkbook([
            { name: 'First', rows: [['a']] },
            { name: 'Second', rows: [['b']] },
        ])

        const sheets = workbookToSheets(workbook)

        expect(sheets.map((sheet) => sheet.name)).toEqual(['First', 'Second'])
    })

    test('행 수가 상한을 넘으면 잘라내고 truncated 를 표시한다', () => {
        const totalRowCount = SPREADSHEET_PREVIEW_MAX_ROWS + 10
        const rows = Array.from({ length: totalRowCount }, (_, index) => [index])
        const workbook = buildWorkbook([{ name: 'Large', rows }])

        const [sheet] = workbookToSheets(workbook)

        expect(sheet.rows).toHaveLength(SPREADSHEET_PREVIEW_MAX_ROWS)
        expect(sheet.totalRowCount).toBe(totalRowCount)
        expect(sheet.truncated).toBe(true)
    })
})
