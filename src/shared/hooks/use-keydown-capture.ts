import { useEffect, useEffectEvent, useSyncExternalStore } from 'react'
import { getKeymapCapturingSnapshot, subscribeKeymapCapturing } from '@shared/lib/keymap-capture'
import { clearKeymapChordState, consumeKeymapMonacoDeferral, getKeymapChordStoreSnapshot } from '@shared/lib/keymap-chord-store'
import { DEFAULT_KEYMAP_CONTEXT_GETTERS, getKeymapContextValue } from '@shared/lib/keymap-context'

/**
 * Also owns the window-`blur` listener that clears chord/monaco-deferral state
 * (`clearKeymapChordState`) — the app losing OS focus mid-chord must release the wait immediately
 * (Wave H contract §3.1), and this hook already owns the one DOM listener lifecycle every
 * `useGlobalKeymap` consumer shares. Registered even while `isCapturing`: the rebind-capture UI's
 * own state is unrelated to (and unaffected by) clearing the *dispatch* chord store.
 *
 * A second `focusin` listener releases only the monaco-deferral window (not the app's own `pending`
 * chord wait) when focus moves *within* the app from the editor to somewhere else — `blur` alone
 * only fires on OS-level focus loss, so armed deferral (Cmd+K observed while an editor was focused)
 * would otherwise survive a same-app focus move (click into the sidebar/terminal) and defer the very
 * next keydown to a monaco instance that no longer has the DOM focus to receive it, silently eating
 * one keystroke. `pending` is left alone here: an app chord wait isn't scoped to editor focus, so a
 * focus move mid-wait (e.g. opening the command palette itself) must not cancel it.
 */
export const useKeydownCapture = (handler: (event: KeyboardEvent) => void) => {
    const isCapturing = useSyncExternalStore(subscribeKeymapCapturing, getKeymapCapturingSnapshot)

    const handleKeyDown = useEffectEvent((event: KeyboardEvent) => {
        if (isCapturing) return
        handler(event)
    })

    const handleWindowBlur = useEffectEvent(() => clearKeymapChordState())

    const handleFocusIn = useEffectEvent(() => {
        if (!getKeymapChordStoreSnapshot().monacoDeferral) return
        if (getKeymapContextValue('editorTextFocus', DEFAULT_KEYMAP_CONTEXT_GETTERS)) return
        consumeKeymapMonacoDeferral()
    })

    useEffect(() => {
        window.addEventListener('keydown', handleKeyDown, true)
        window.addEventListener('blur', handleWindowBlur)
        document.addEventListener('focusin', handleFocusIn)
        return () => {
            window.removeEventListener('keydown', handleKeyDown, true)
            window.removeEventListener('blur', handleWindowBlur)
            document.removeEventListener('focusin', handleFocusIn)
        }
    }, [])
}
