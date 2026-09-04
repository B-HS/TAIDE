import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ProjectDisplayPatch, ProjectRef } from '@shared/api/bindings'
import { PROJECT_COLOR_TOKENS, PROJECT_COLOR_VAR_PREFIX, PROJECT_LABEL_MAX_CODEPOINTS } from '@shared/constants/project-display'
import type { ProjectColorToken } from '@shared/constants/project-display'
import { cn } from '@shared/lib/cn'
import type { ProjectDisplayMode } from '@shared/lib/project-display'
import { clampProjectLabel, normalizeProjectLabel, resolveProjectColorToken, resolveProjectDisplay } from '@shared/lib/project-display'
import {
    DEFAULT_PROJECT_ICON_NAME,
    PROJECT_ICON_COMPONENT_MAP,
    PROJECT_ICON_NAMES,
    resolveProjectIconName,
} from '@shared/icons/project-icon-registry'
import type { ProjectIconName } from '@shared/icons/project-icon-registry'
import { Button } from '@shared/ui/button'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@shared/ui/command'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { ProjectDisplayGlyph } from '@features/project/project-display-glyph'

type ProjectDisplayDialogProps = {
    open: boolean
    projectName: string
    display: ProjectRef['display']
    isPending: boolean
    onOpenChange: (open: boolean) => void
    onSubmit: (patch: ProjectDisplayPatch) => void
}

const DISPLAY_MODE_ORDER: readonly ProjectDisplayMode[] = ['icon', 'label', 'default']

const DISPLAY_MODE_LABEL_KEY: Record<ProjectDisplayMode, string> = {
    icon: 'project.displayModeIcon',
    label: 'project.displayModeLabel',
    default: 'project.displayModeDefault',
}

/**
 * One `name` for every instance is safe: the dialog renders nothing while closed (radix mounts the
 * portal only when `open`), so two mode radio groups can never coexist even though the sidebar keeps
 * one dialog per project row.
 */
const DISPLAY_MODE_RADIO_NAME = 'project-display-mode'

/**
 * Edits one project's sidebar presentation. The three modes are exclusive because they compete for
 * the same 40px button — a color, which only tints the glyph, stays selectable in all three.
 *
 * The dialog stays mounted across opens (only `open` toggles), so every field is reset on the
 * closed→open transition *during render*, the same guard `create-tag-dialog.tsx` uses: without it a
 * cancelled edit would reappear on the next project's dialog, one Save away from being written to
 * the wrong project.
 *
 * Save always sends all three axes — `''` for the ones this mode does not use — because
 * `ProjectDisplayPatch` reads `null` as "leave alone". Sending `null` for the unused axes would let
 * a project keep the icon it had while showing a label, and "reset to default" would clear nothing.
 */
export const ProjectDisplayDialog: FC<ProjectDisplayDialogProps> = ({ open, projectName, display, isPending, onOpenChange, onSubmit }) => {
    const [mode, setMode] = useState<ProjectDisplayMode>('default')
    const [label, setLabel] = useState('')
    const [iconName, setIconName] = useState<ProjectIconName>(DEFAULT_PROJECT_ICON_NAME)
    const [colorToken, setColorToken] = useState<ProjectColorToken | null>(null)
    const [wasOpen, setWasOpen] = useState(open)

    if (wasOpen !== open) {
        setWasOpen(open)
        if (open) {
            setMode(resolveProjectDisplay({ display }).mode)
            setLabel(clampProjectLabel(display?.label ?? ''))
            setIconName(resolveProjectIconName(display?.icon))
            setColorToken(resolveProjectColorToken(display?.color))
        }
    }

    const normalizedLabel = normalizeProjectLabel(label)
    const preview = resolveProjectDisplay({
        display: { icon: mode === 'icon' ? iconName : null, label: mode === 'label' ? normalizedLabel : null, color: colorToken },
    })
    const canSave = mode !== 'label' || normalizedLabel.length > 0

    const handleSave = () =>
        onSubmit({ icon: mode === 'icon' ? iconName : '', label: mode === 'label' ? normalizedLabel : '', color: colorToken ?? '' })

    const { t } = useTranslation()

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{t('project.displayDialogTitle')}</DialogTitle>
                </DialogHeader>

                <div className='flex flex-col gap-4'>
                    <div role='radiogroup' aria-label={t('project.displayDialogTitle')} className='flex items-center gap-4 text-xs'>
                        {DISPLAY_MODE_ORDER.map((candidate) => (
                            <label key={candidate} className='flex items-center gap-1.5'>
                                <input
                                    type='radio'
                                    name={DISPLAY_MODE_RADIO_NAME}
                                    value={candidate}
                                    checked={mode === candidate}
                                    onChange={() => setMode(candidate)}
                                />
                                <span className='text-app-foreground'>{t(DISPLAY_MODE_LABEL_KEY[candidate])}</span>
                            </label>
                        ))}
                    </div>

                    {mode === 'icon' && (
                        <div className='border-app-border overflow-hidden rounded-md border'>
                            <Command className='text-app-foreground bg-transparent'>
                                <CommandInput placeholder={t('project.displayIconSearch')} className='text-xs' />
                                <CommandList className='max-h-44'>
                                    <CommandEmpty className='text-app-sidebar-icon-default py-4 text-center text-xs'>
                                        {t('palette.noResults')}
                                    </CommandEmpty>
                                    <CommandGroup className='[&_[cmdk-group-items]]:grid [&_[cmdk-group-items]]:grid-cols-8 [&_[cmdk-group-items]]:gap-1'>
                                        {PROJECT_ICON_NAMES.map((candidate) => {
                                            const Icon = PROJECT_ICON_COMPONENT_MAP[candidate]
                                            return (
                                                <CommandItem
                                                    key={candidate}
                                                    value={candidate}
                                                    aria-label={candidate}
                                                    onSelect={() => setIconName(candidate)}
                                                    className={cn(
                                                        'flex aspect-square items-center justify-center p-0',
                                                        iconName === candidate && 'ring-app-accent bg-app-sidebar-item-active ring-1 ring-inset',
                                                    )}>
                                                    <Icon aria-hidden className='text-app-foreground size-4' />
                                                </CommandItem>
                                            )
                                        })}
                                    </CommandGroup>
                                </CommandList>
                            </Command>
                        </div>
                    )}

                    {mode === 'label' && (
                        <label className='flex items-center gap-3 text-xs'>
                            <span className='text-app-foreground shrink-0'>{t('project.displayLabelField')}</span>
                            <input
                                autoFocus
                                value={label}
                                maxLength={PROJECT_LABEL_MAX_CODEPOINTS}
                                onChange={(event) => setLabel(clampProjectLabel(event.target.value))}
                                className='bg-panel-input-background border-panel-input-border text-app-foreground w-24 rounded-sm border px-2 py-1 text-center outline-none'
                            />
                            <span className='text-app-sidebar-icon-default'>{t('project.displayLabelHint')}</span>
                        </label>
                    )}

                    <div role='group' aria-label={t('project.displayColor')} className='flex flex-col gap-2 text-xs'>
                        <span className='text-app-foreground'>{t('project.displayColor')}</span>
                        <div className='flex flex-wrap gap-1.5'>
                            {PROJECT_COLOR_TOKENS.map((token, index) => (
                                <button
                                    key={token}
                                    type='button'
                                    aria-label={`${t('project.displayColor')} ${index + 1}`}
                                    aria-pressed={colorToken === token}
                                    onClick={() => setColorToken(colorToken === token ? null : token)}
                                    style={{ backgroundColor: `var(${PROJECT_COLOR_VAR_PREFIX}${token})` }}
                                    className={cn(
                                        'border-app-border size-6 rounded-full border',
                                        colorToken === token && 'ring-app-accent ring-2 ring-offset-1',
                                    )}
                                />
                            ))}
                        </div>
                    </div>

                    <div className='flex items-center gap-3 text-xs'>
                        <span className='text-app-sidebar-icon-default'>{t('project.displayPreview')}</span>
                        <div className='bg-app-sidebar-background text-app-sidebar-icon-default flex size-10 shrink-0 items-center justify-center rounded-md'>
                            <span className='flex max-w-full items-center justify-center overflow-hidden'>
                                <ProjectDisplayGlyph display={preview} />
                            </span>
                        </div>
                        <span className='text-app-foreground min-w-0 truncate'>{projectName}</span>
                    </div>
                </div>

                <DialogFooter>
                    <Button type='button' variant='outline' onClick={() => onOpenChange(false)}>
                        {t('common.cancel')}
                    </Button>
                    <Button type='button' disabled={!canSave || isPending} onClick={handleSave}>
                        {t('common.save')}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}
