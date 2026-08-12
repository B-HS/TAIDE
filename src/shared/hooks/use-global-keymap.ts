import type { KeymapActionId, KeymapEntry } from '@shared/lib/keymap'
import { APP_KEYMAP, findMatchingKeymapEntry } from '@shared/lib/keymap'
import { useKeydownCapture } from '@shared/hooks/use-keydown-capture'

export type KeymapHandlers = Partial<Record<KeymapActionId, () => void>>

export const useGlobalKeymap = (handlers: KeymapHandlers, entries: KeymapEntry[] = APP_KEYMAP) => {
    useKeydownCapture((event) => {
        const entry = findMatchingKeymapEntry(entries, event)
        if (!entry) return
        const handler = handlers[entry.id]
        if (!handler) return
        event.preventDefault()
        handler()
    })
}
