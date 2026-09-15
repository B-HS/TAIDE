import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@shared/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'

const INPUT_CLASS_NAME = 'bg-panel-input-background border-panel-input-border text-app-foreground rounded-sm border px-2 py-1 text-sm outline-none'

type OpenProjectByPathDialogProps = {
    open: boolean
    isPending: boolean
    onOpenChange: (open: boolean) => void
    onConfirm: (path: string) => void
}

/**
 * Types a project folder path instead of picking it in Finder — the only entry point that reaches a
 * path no file dialog can express (a `~` shorthand, a server mount, a path pasted from a terminal).
 * The value is handed to `project_open` as typed except for trimming; `~` expansion and existence
 * checks belong to Rust (`domain::project::service::resolve_open_root`), so this dialog never has to
 * guess what a path means on the current platform.
 *
 * Like `create-tag-dialog.tsx`, the dialog stays mounted across opens and only `open` toggles, so
 * the input is cleared on the closed to open transition during render — a cancelled path must not
 * reappear one confirm away from opening the wrong folder.
 */
export const OpenProjectByPathDialog: FC<OpenProjectByPathDialogProps> = ({ open, isPending, onOpenChange, onConfirm }) => {
    const { t } = useTranslation()
    const [path, setPath] = useState('')
    const [wasOpen, setWasOpen] = useState(open)

    if (wasOpen !== open) {
        setWasOpen(open)
        if (open) setPath('')
    }

    const trimmedPath = path.trim()

    const handleConfirm = () => {
        if (!trimmedPath || isPending) return
        onConfirm(trimmedPath)
    }

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{t('sidebar.openByPathTitle')}</DialogTitle>
                </DialogHeader>
                <input
                    autoFocus
                    value={path}
                    onChange={(event) => setPath(event.target.value)}
                    onKeyDown={(event) => event.key === 'Enter' && handleConfirm()}
                    placeholder={t('sidebar.openByPathPlaceholder')}
                    className={INPUT_CLASS_NAME}
                />
                <DialogFooter>
                    <Button type='button' variant='outline' onClick={() => onOpenChange(false)}>
                        {t('common.cancel')}
                    </Button>
                    <Button type='button' disabled={!trimmedPath || isPending} onClick={handleConfirm}>
                        {t('app.openProject')}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}
