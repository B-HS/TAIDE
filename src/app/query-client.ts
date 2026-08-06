import { QueryClient, focusManager, onlineManager } from '@tanstack/react-query'
import { getCurrentWindow } from '@tauri-apps/api/window'

const STALE_TIME_MS = 60_000
const GC_TIME_MS = 10 * 60_000

export const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            staleTime: STALE_TIME_MS,
            gcTime: GC_TIME_MS,
            retry: 0,
            networkMode: 'always',
            refetchOnWindowFocus: false,
        },
        mutations: { networkMode: 'always', retry: 0 },
    },
})

export const bindQueryClientToWindow = () => {
    onlineManager.setOnline(true)
    focusManager.setEventListener((handleFocus) => {
        const unlisten = getCurrentWindow().onFocusChanged(({ payload }) => handleFocus(payload))
        return () => {
            void unlisten.then((dispose) => dispose())
        }
    })
}
