import type { FC } from 'react'
import { useState } from 'react'
import { FileWarning, TableProperties } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@shared/lib/cn'
import { parseSpreadsheetWorkbook, workbookToSheets } from '@shared/lib/spreadsheet'
import type { SpreadsheetSheet } from '@shared/lib/spreadsheet'
import { ScrollContainer } from '@shared/scroll/scroll-container'
import { PreviewStatusMessage } from '@features/preview/preview-status'

export type SpreadsheetPreviewProps = {
    data: ArrayBuffer
    onOpenExternally: () => void
}

const parseSheets = (data: ArrayBuffer): SpreadsheetSheet[] | null => {
    try {
        return workbookToSheets(parseSpreadsheetWorkbook(data))
    } catch {
        return null
    }
}

const formatCell = (value: SpreadsheetSheet['rows'][number][number]) => (value === null ? '' : String(value))

export const SpreadsheetPreview: FC<SpreadsheetPreviewProps> = ({ data, onOpenExternally }) => {
    const [activeSheetIndex, setActiveSheetIndex] = useState(0)

    const { t } = useTranslation()

    const sheets = parseSheets(data)

    if (sheets === null) {
        return (
            <PreviewStatusMessage
                icon={<FileWarning className='size-5' />}
                message={t('preview.spreadsheet.loadFailed')}
                actionLabel={t('preview.openExternally')}
                onAction={onOpenExternally}
            />
        )
    }

    if (sheets.length === 0) {
        return <PreviewStatusMessage icon={<TableProperties className='size-5' />} message={t('preview.spreadsheet.noSheets')} />
    }

    const safeActiveSheetIndex = Math.min(activeSheetIndex, sheets.length - 1)
    const activeSheet = sheets[safeActiveSheetIndex]

    return (
        <div className='bg-editor-background flex h-full w-full flex-col'>
            <div
                role='tablist'
                className='border-editor-widget-border bg-editor-widget-background flex shrink-0 items-center gap-1 overflow-x-auto border-b px-2 py-1'>
                {sheets.map((sheet, index) => (
                    <button
                        key={sheet.name}
                        type='button'
                        role='tab'
                        aria-selected={index === safeActiveSheetIndex}
                        onClick={() => setActiveSheetIndex(index)}
                        className={cn(
                            'shrink-0 rounded-sm px-2 py-1 text-xs whitespace-nowrap',
                            index === safeActiveSheetIndex
                                ? 'bg-tab-bar-tab-active-background text-tab-bar-tab-active-foreground'
                                : 'text-tab-bar-tab-inactive-foreground hover:bg-explorer-item-hover',
                        )}>
                        {sheet.name}
                    </button>
                ))}
            </div>

            {activeSheet.truncated && (
                <div className='text-status-warning bg-editor-widget-background border-editor-widget-border shrink-0 border-b px-3 py-1 text-xs'>
                    {t('preview.spreadsheet.truncatedNotice', { shown: activeSheet.rows.length, total: activeSheet.totalRowCount })}
                </div>
            )}

            <ScrollContainer className='flex-1' orientation='both'>
                {activeSheet.rows.length === 0 ? (
                    <PreviewStatusMessage icon={<TableProperties className='size-5' />} message={t('preview.spreadsheet.emptySheet')} />
                ) : (
                    <table className='border-app-border w-full border-collapse text-xs'>
                        <tbody>
                            {activeSheet.rows.map((row, rowIndex) => (
                                <tr key={rowIndex} className='even:bg-editor-widget-background'>
                                    {row.map((cell, cellIndex) => (
                                        <td key={cellIndex} className='border-app-border text-editor-foreground border px-2 py-1 whitespace-nowrap'>
                                            {formatCell(cell)}
                                        </td>
                                    ))}
                                </tr>
                            ))}
                        </tbody>
                    </table>
                )}
            </ScrollContainer>
        </div>
    )
}
