import { useEffect, useEffectEvent, useSyncExternalStore } from 'react'
import { getKeymapCapturingSnapshot, subscribeKeymapCapturing } from '@shared/lib/keymap-capture'

export const useKeydownCapture = (handler: (event: KeyboardEvent) => void) => {
    const isCapturing = useSyncExternalStore(subscribeKeymapCapturing, getKeymapCapturingSnapshot)

    const handleKeyDown = useEffectEvent((event: KeyboardEvent) => {
        if (isCapturing) return
        handler(event)
    })

    useEffect(() => {
        window.addEventListener('keydown', handleKeyDown, true)
        return () => window.removeEventListener('keydown', handleKeyDown, true)
    }, [])
}
