import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { PROJECT_COLOR_TOKENS, PROJECT_COLOR_VAR_PREFIX } from '@shared/constants/project-display'
import { PROJECT_GROUP_NAME_MAX_CODEPOINTS } from '@shared/constants/project-group'
import { cn } from '@shared/lib/cn'
import { resolveProjectColorToken } from '@shared/lib/project-display'
import { clampProjectGroupName, normalizeProjectGroupName } from '@shared/lib/project-group'
import { Button } from '@shared/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'

/** Which wording the same form wears. Creating and editing a group differ in the title and the confirm label only — the two fields, their limits, and the write they produce are identical. */
export type ProjectGroupDialogMode = 'create' | 'edit'

const TITLE_KEY: Record<ProjectGroupDialogMode, string> = { create: 'projectGroup.createTitle', edit: 'projectGroup.editTitle' }

const CONFIRM_KEY: Record<ProjectGroupDialogMode, string> = { create: 'projectGroup.create', edit: 'common.save' }

type ProjectGroupDialogProps = {
    open: boolean
    mode: ProjectGroupDialogMode
    initialName: string
    initialColor: string | null
    isPending: boolean
    onOpenChange: (open: boolean) => void
    onSubmit: (value: { name: string; color: string | null }) => void
}

/**
 * Names and tints one sidebar group, following `project-display-dialog.tsx` in both shape and
 * palette: the twelve swatches are that dialog's own `PROJECT_COLOR_TOKENS`, which is also the
 * allow-list `domain::project::service::sanitize_group_color` validates against, so a group header
 * and a project icon can be tinted from the same theme tokens and this side can never compose a
 * color the backend refuses.
 *
 * Every field is reset on the closed to open transition *during render* — the same guard
 * `project-display-dialog.tsx` and `create-tag-dialog.tsx` use. The dialog stays mounted between
 * opens, so without it a cancelled rename would reappear on the next group's dialog, one confirm
 * away from being written to the wrong group.
 */
export const ProjectGroupDialog: FC<ProjectGroupDialogProps> = ({ open, mode, initialName, initialColor, isPending, onOpenChange, onSubmit }) => {
    const { t } = useTranslation()
    const [name, setName] = useState('')
    const [color, setColor] = useState<string | null>(null)
    const [wasOpen, setWasOpen] = useState(open)

    if (wasOpen !== open) {
        setWasOpen(open)
        if (open) {
            setName(clampProjectGroupName(initialName))
            setColor(resolveProjectColorToken(initialColor))
        }
    }

    const normalizedName = normalizeProjectGroupName(name)

    const handleConfirm = () => {
        if (!normalizedName || isPending) return
        onSubmit({ name: normalizedName, color })
    }

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{t(TITLE_KEY[mode])}</DialogTitle>
                </DialogHeader>

                <div className='flex flex-col gap-4'>
                    <label className='flex items-center gap-3 text-xs'>
                        <span className='text-app-foreground shrink-0'>{t('projectGroup.nameField')}</span>
                        <input
                            autoFocus
                            value={name}
                            maxLength={PROJECT_GROUP_NAME_MAX_CODEPOINTS}
                            onChange={(event) => setName(clampProjectGroupName(event.target.value))}
                            onKeyDown={(event) => event.key === 'Enter' && handleConfirm()}
                            className='bg-panel-input-background border-panel-input-border text-app-foreground min-w-0 flex-1 rounded-sm border px-2 py-1 outline-none'
                        />
                    </label>

                    <div role='group' aria-label={t('projectGroup.color')} className='flex flex-col gap-2 text-xs'>
                        <span className='text-app-foreground'>{t('projectGroup.color')}</span>
                        <div className='flex flex-wrap gap-1.5'>
                            {PROJECT_COLOR_TOKENS.map((token, index) => (
                                <button
                                    key={token}
                                    type='button'
                                    aria-label={`${t('projectGroup.color')} ${index + 1}`}
                                    aria-pressed={color === token}
                                    onClick={() => setColor(color === token ? null : token)}
                                    style={{ backgroundColor: `var(${PROJECT_COLOR_VAR_PREFIX}${token})` }}
                                    className={cn(
                                        'border-app-border size-6 rounded-full border',
                                        color === token && 'ring-app-accent ring-2 ring-offset-1',
                                    )}
                                />
                            ))}
                        </div>
                    </div>
                </div>

                <DialogFooter>
                    <Button type='button' variant='outline' onClick={() => onOpenChange(false)}>
                        {t('common.cancel')}
                    </Button>
                    <Button type='button' disabled={!normalizedName || isPending} onClick={handleConfirm}>
                        {t(CONFIRM_KEY[mode])}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}
