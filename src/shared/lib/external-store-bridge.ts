export type ExternalStoreBridge<Value> = {
    setValue: (value: Value) => void
    getSnapshot: () => Value
    subscribe: (listener: () => void) => () => void
}

/**
 * Creates a module-scope value cell whose `{ subscribe, getSnapshot }` pair matches React's
 * `useSyncExternalStore(subscribe, getSnapshot)` contract directly — for client-only state that
 * must also be readable synchronously outside React, and has no server counterpart (a pending
 * LSP install progress value, a live theme preview override). It intentionally sits below
 * TanStack Query (that owns server state — see query.md) and below zustand (that owns
 * higher-traffic cross-widget state — see frontend.md §6): reach for it only when a value is
 * exactly "one current snapshot + change notifications" and nothing more.
 *
 * Usage:
 * ```ts
 * const themePreviewStore = createExternalStoreBridge<ThemePreview | null>(null)
 * export const setThemePreview = themePreviewStore.setValue
 * export const useThemePreview = () => useSyncExternalStore(themePreviewStore.subscribe, themePreviewStore.getSnapshot)
 * ```
 */
export const createExternalStoreBridge = <Value>(initialValue: Value): ExternalStoreBridge<Value> => {
    let value = initialValue
    const listeners = new Set<() => void>()

    const setValue = (nextValue: Value) => {
        value = nextValue
        for (const listener of listeners) listener()
    }

    const getSnapshot = () => value

    const subscribe = (listener: () => void) => {
        listeners.add(listener)
        return () => {
            listeners.delete(listener)
        }
    }

    return { setValue, getSnapshot, subscribe }
}
