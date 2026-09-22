import { expect, spyOn, test } from 'bun:test'
import type { EventCallback } from '@tauri-apps/api/event'
import type { Settings, SettingsChanged } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, renderWithProviders } from '@shared/testing/render'

test('다른 창의 설정 변경 이벤트가 언어와 테마 캐시를 다시 읽게 한다', async () => {
    const original = window.__TAURI_INTERNALS__
    window.__TAURI_INTERNALS__ = { transformCallback: () => 1, invoke: () => Promise.resolve(1) }
    Reflect.set(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', { unregisterListener: () => undefined })
    let deliver: EventCallback<SettingsChanged> | undefined
    spyOn(events.settingsChanged, 'listen').mockImplementation((handler) => {
        deliver = handler
        return Promise.resolve(() => undefined)
    })
    const settings: Settings = { version: 1, language: 'ko', themeId: 'dark-plus', followSystemTheme: false }
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.SETTINGS.CURRENT, queryFn: () => settings, gcTime: Infinity })
    for (const queryKey of [QUERY_KEY.THEME.CURRENT, QUERY_KEY.LOCALE.CURRENT, QUERY_KEY.REMOTE.STATUS]) {
        await queryClient.fetchQuery({ queryKey, queryFn: () => 'seed', gcTime: Infinity })
    }
    const { IpcSyncProvider } = await import('@app/providers/ipc-sync-provider')
    const rendered = renderWithProviders(<IpcSyncProvider />, { queryClient })
    try {
        await act(async () => {
            deliver?.({ event: 'settings:changed', id: 1, payload: { settings: { ...settings, language: 'en', followSystemTheme: true } } })
        })
        expect(queryClient.getQueryState(QUERY_KEY.LOCALE.CURRENT)?.isInvalidated).toBe(true)
        expect(queryClient.getQueryState(QUERY_KEY.THEME.CURRENT)?.isInvalidated).toBe(true)
        expect(queryClient.getQueryState(QUERY_KEY.REMOTE.STATUS)?.isInvalidated).toBe(false)
        expect(queryClient.getQueryData<Settings>(QUERY_KEY.SETTINGS.CURRENT)?.language).toBe('en')
    } finally {
        rendered.unmount()
        window.__TAURI_INTERNALS__ = original
        Reflect.deleteProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__')
    }
})
