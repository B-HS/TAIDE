import { useEffect, useEffectEvent } from 'react'
import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event'

type TauriEvent<T> = {
    listen: (handler: EventCallback<T>) => Promise<UnlistenFn>
}

export const useTauriEvent = <T>(event: TauriEvent<T>, handler: EventCallback<T>) => {
    const stableHandler = useEffectEvent(handler)

    useEffect(() => {
        let disposed = false
        let dispose: UnlistenFn | undefined

        void event
            .listen((payload) => stableHandler(payload))
            .then((unlisten) => {
                if (disposed) {
                    unlisten()
                    return
                }
                dispose = unlisten
            })

        return () => {
            disposed = true
            dispose?.()
        }
    }, [event])
}
