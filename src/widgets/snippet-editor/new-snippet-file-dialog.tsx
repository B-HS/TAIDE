import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { TAIDE_LANGUAGE_IDS } from '@shared/lib/shiki/lang-map'
import { buildLanguageSnippetFileName, isSafeSnippetFileName, normalizeGlobalSnippetFileName } from '@shared/lib/snippet-draft'
import { OptionPicker } from '@features/settings/option-picker'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { Button } from '@shared/ui/button'

type NewSnippetFileDialogProps = {
    open: boolean
    existingFileNames: readonly string[]
    onOpenChange: (open: boolean) => void
    onCreate: (fileName: string) => void
}

const GLOBAL_OPTION_ID = 'global'

const INPUT_CLASS_NAME = 'bg-panel-input-background border-panel-input-border text-app-foreground rounded-sm border px-2 py-1 text-sm outline-none'

export const NewSnippetFileDialog: FC<NewSnippetFileDialogProps> = ({ open, existingFileNames, onOpenChange, onCreate }) => {
    const { t } = useTranslation()
    const [selectedOptionId, setSelectedOptionId] = useState<string>(TAIDE_LANGUAGE_IDS[0])
    const [globalName, setGlobalName] = useState('')

    const isGlobal = selectedOptionId === GLOBAL_OPTION_ID
    const fileName = isGlobal ? normalizeGlobalSnippetFileName(globalName) : buildLanguageSnippetFileName(selectedOptionId)
    const canCreate = (isGlobal ? globalName.trim().length > 0 && isSafeSnippetFileName(fileName) : true) && !existingFileNames.includes(fileName)

    const options = [
        { id: GLOBAL_OPTION_ID, label: t('snippetEditor.newFileGlobalOption') },
        ...TAIDE_LANGUAGE_IDS.map((languageId) => ({ id: languageId, label: languageId })),
    ]

    const handleConfirm = () => {
        if (!canCreate) return
        onCreate(fileName)
    }

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{t('snippetEditor.newFileDialogTitle')}</DialogTitle>
                </DialogHeader>
                <div className='flex flex-col gap-2'>
                    <OptionPicker
                        label={t('snippetEditor.newFileLanguagePlaceholder')}
                        options={options}
                        value={selectedOptionId}
                        onSelect={setSelectedOptionId}
                    />
                    {isGlobal && (
                        <input
                            autoFocus
                            value={globalName}
                            onChange={(event) => setGlobalName(event.target.value)}
                            placeholder={t('snippetEditor.newFileGlobalNamePlaceholder')}
                            aria-label={t('snippetEditor.newFileGlobalNamePlaceholder')}
                            className={INPUT_CLASS_NAME}
                        />
                    )}
                </div>
                <DialogFooter>
                    <Button type='button' variant='outline' onClick={() => onOpenChange(false)}>
                        {t('common.cancel')}
                    </Button>
                    <Button type='button' disabled={!canCreate} onClick={handleConfirm}>
                        {t('common.confirm')}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}
