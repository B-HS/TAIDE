import { useState } from 'react'
import { createRoot } from 'react-dom/client'
import { createInstance } from 'i18next'
import { initReactI18next, I18nextProvider } from 'react-i18next'
import type { FC } from 'react'
import { FileTreeDraftRowItem } from '@features/explorer/file-tree-draft-row'
import { Dialog, DialogContent, DialogTitle } from '@shared/ui/dialog'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@shared/ui/tooltip'
import '@shared/styles/global.css'

const TRIGGER_LEFT = 80
const TRIGGER_TOP = 100
const TRIGGER_WIDTH = 40
const TRIGGER_HEIGHT = 20
const INPUT_LEFT = 240
const INPUT_TOP = 200
const INPUT_WIDTH = 160
const INPUT_HEIGHT = 24
const DELAY_MS = 400
const MODE = new URL(window.location.href).searchParams.get('mode')
const LOCALE = {
    common: { close: 'Close' },
    explorer: { entryNamePlaceholder: 'Synthetic draft' },
    fixture: {
        trigger: 'Reference',
        tooltip: 'Normal tooltip',
        error: 'Validation error',
        dialog: 'Synthetic modal',
        open: 'Open modal',
        commit: 'Commit',
    },
}
const i18n = createInstance()
await i18n.use(initReactI18next).init({ lng: 'en', resources: { en: { translation: LOCALE } }, interpolation: { escapeValue: false } })

export const TooltipEscapeReference: FC = () => {
    const [hasDraft, setHasDraft] = useState(MODE !== 'modal')
    const [isDialogOpen, setIsDialogOpen] = useState(false)
    const [records, setRecords] = useState<string[]>([])
    const handleRecord = (record: string) => setRecords((previous) => [...previous, record])
    const handleCancel = () => {
        handleRecord('draft:cancel')
        setHasDraft(false)
    }
    const handleDialog = (isOpen: boolean) => {
        handleRecord(`dialog:${isOpen ? 'open' : 'closed'}`)
        setIsDialogOpen(isOpen)
    }
    return (
        <I18nextProvider i18n={i18n}>
            <TooltipProvider delayDuration={DELAY_MS}>
                <output data-records>{records.join('|')}</output>
                <output data-dialog-state>{isDialogOpen ? 'open' : 'closed'}</output>
                <Tooltip onOpenChange={(isOpen) => handleRecord(`normal:${isOpen ? 'open' : 'closed'}`)}>
                    <TooltipTrigger asChild>
                        <button
                            type='button'
                            data-escape-trigger
                            style={{ position: 'fixed', left: TRIGGER_LEFT, top: TRIGGER_TOP, width: TRIGGER_WIDTH, height: TRIGGER_HEIGHT }}>
                            {i18n.t('fixture.trigger')}
                        </button>
                    </TooltipTrigger>
                    <TooltipContent data-escape-content>{i18n.t('fixture.tooltip')}</TooltipContent>
                </Tooltip>
                {hasDraft && (
                    <FileTreeDraftRowItem
                        depth={0}
                        kind='file'
                        initialName='draft.txt'
                        error={MODE === 'invalid' ? i18n.t('fixture.error') : null}
                        style={{ position: 'fixed', left: INPUT_LEFT, top: INPUT_TOP, width: INPUT_WIDTH, height: INPUT_HEIGHT }}
                        onCancel={handleCancel}
                        onCommit={() => handleRecord('draft:commit')}
                    />
                )}
                {MODE === 'modal' && (
                    <Dialog open={isDialogOpen} onOpenChange={handleDialog}>
                        <button type='button' data-open-dialog onClick={() => handleDialog(true)}>
                            {i18n.t('fixture.open')}
                        </button>
                        <DialogContent showCloseButton={false} aria-describedby={undefined}>
                            <DialogTitle>{i18n.t('fixture.dialog')}</DialogTitle>
                            <button type='button'>{i18n.t('fixture.commit')}</button>
                        </DialogContent>
                    </Dialog>
                )}
            </TooltipProvider>
        </I18nextProvider>
    )
}

const root = document.getElementById('root')
if (!root) throw new Error('Missing synthetic tooltip Escape root')
createRoot(root).render(<TooltipEscapeReference />)
