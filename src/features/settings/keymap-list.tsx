import type { FC, KeyboardEvent } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { KeymapActionId, KeymapEntry, KeymapModifier } from '@shared/lib/keymap'
import { captureModsFromEvent, formatKeymapShortcut, normalizeKeymapKey } from '@shared/lib/keymap'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'

const MODIFIER_ONLY_KEYS = ['Shift', 'Control', 'Alt', 'Meta']

type KeymapListProps = {
    entries: KeymapEntry[]
    overriddenActionIds: KeymapActionId[]
    onChangeBinding: (actionId: KeymapActionId, key: string, mods: KeymapModifier[]) => void
}

export const KeymapList: FC<KeymapListProps> = ({ entries, overriddenActionIds, onChangeBinding }) => {
    const [capturingActionId, setCapturingActionId] = useState<KeymapActionId | null>(null)

    const { t } = useTranslation()

    const handleCaptureKeyDown = (event: KeyboardEvent<HTMLButtonElement>, actionId: KeymapActionId) => {
        event.preventDefault()
        if (event.key === 'Escape') {
            setCapturingActionId(null)
            return
        }
        if (MODIFIER_ONLY_KEYS.includes(event.key)) return

        onChangeBinding(actionId, normalizeKeymapKey(event.key), captureModsFromEvent(event))
        setCapturingActionId(null)
    }

    return (
        <ul className='flex flex-col gap-1'>
            {entries.map((entry) => {
                const isCapturing = capturingActionId === entry.id
                const isOverridden = overriddenActionIds.includes(entry.id)

                return (
                    <li
                        key={entry.id}
                        className='border-app-border flex min-w-0 items-center justify-between gap-3 rounded-sm border px-3 py-1.5 text-xs'>
                        <span className='flex min-w-0 items-center gap-2'>
                            <span className='text-app-foreground truncate'>{t(entry.descriptionKey)}</span>
                            {isOverridden && <span className='text-app-sidebar-icon-default shrink-0'>{t('settings.keymapCustomized')}</span>}
                        </span>
                        {isCapturing ? (
                            <button
                                type='button'
                                autoFocus
                                onKeyDown={(event) => handleCaptureKeyDown(event, entry.id)}
                                onBlur={() => setCapturingActionId(null)}
                                className='border-app-focus-border bg-app-sidebar-item-active text-app-foreground shrink-0 rounded-sm border px-2 py-1 font-mono'>
                                {t('settings.keymapCapturePrompt')}
                            </button>
                        ) : (
                            <div className='flex shrink-0 items-center gap-2'>
                                <span className={cn('text-app-foreground font-mono', isOverridden && 'text-app-accent')}>
                                    {formatKeymapShortcut(entry)}
                                </span>
                                <Button type='button' variant='outline' size='xs' onClick={() => setCapturingActionId(entry.id)}>
                                    {t('settings.keymapChange')}
                                </Button>
                            </div>
                        )}
                    </li>
                )
            })}
        </ul>
    )
}
