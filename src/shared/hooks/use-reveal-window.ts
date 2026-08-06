import { useEffect } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'

export const useRevealWindow = (ready: boolean) => {
    useEffect(() => {
        if (!ready) return
        void getCurrentWindow().show()
    }, [ready])
}
