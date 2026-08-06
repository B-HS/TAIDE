import { useEffect, useEffectEvent } from 'react'
import type { KeymapActionId, KeymapEntry } from '@shared/lib/keymap'
import { APP_KEYMAP, findMatchingKeymapEntry } from '@shared/lib/keymap'

export type KeymapHandlers = Partial<Record<KeymapActionId, () => void>>

export const useGlobalKeymap = (handlers: KeymapHandlers, entries: KeymapEntry[] = APP_KEYMAP) => {
    const handleKeyDown = useEffectEvent((event: KeyboardEvent) => {
        const entry = findMatchingKeymapEntry(entries, event)
        if (!entry) return
        const handler = handlers[entry.id]
        if (!handler) return
        event.preventDefault()
        handler()
    })

    useEffect(() => {
        window.addEventListener('keydown', handleKeyDown, true)
        return () => window.removeEventListener('keydown', handleKeyDown, true)
    }, [])
}
