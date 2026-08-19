import { describe, expect, mock, test } from 'bun:test'
import { QueryClient } from '@tanstack/react-query'
import type { Settings } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'

/**
 * `@shared/lib/monaco/setup` pulls in real monaco-editor worker bundles (`?worker` imports) that
 * `bun test` cannot resolve at all — `lsp-session-registry.test.ts` documents and works around the
 * same issue. `@entities/lsp/lsp.ipc` re-exports Tauri command bindings this environment cannot load
 * either. Both are mocked here for the same reasons those files mock them (`mock.module` is
 * process-global and last-registration-wins, so this fake covers each module's entire export
 * surface rather than only what this file's own tests touch), and `use-lsp-session.ts` — which
 * transitively imports both, directly and via `lsp-session-registry.ts` — is reached through a
 * *dynamic* `import()` so the mocks are registered before its static import graph resolves.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: {} }))
mock.module('@entities/lsp/lsp.ipc', () => ({
    spawnLspSession: () => Promise.resolve('fake-session'),
    sendLspMessage: () => Promise.resolve(),
    stopLspSession: () => Promise.resolve(),
    restartLspSession: () => Promise.resolve(),
    confirmLspReinitialize: () => Promise.resolve(),
    listLspSessions: () => Promise.resolve([]),
    detectLspServers: () => Promise.resolve([]),
    resolveLspRoot: () => Promise.resolve(null),
    installLspServer: () => Promise.resolve(),
    cancelLspInstall: () => Promise.resolve(),
}))

const importUseLspSession = () => import('@widgets/editor-pane/use-lsp-session')

const settingsWith = (patch: Partial<Settings>): Settings => ({ version: 1, ...patch })

describe('observeSemanticHighlightingSetting — F3#18 저수준 캐시 구독 교체 동등성 (contract §1.2)', () => {
    test('editorSemanticHighlighting 값이 실제로 바뀔 때만 콜백을 호출한다 — 무관한 필드 변경은 무시', async () => {
        const { observeSemanticHighlightingSetting } = await importUseLspSession()
        const queryClient = new QueryClient()
        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({ editorSemanticHighlighting: true }))

        let callCount = 0
        const unsubscribe = observeSemanticHighlightingSetting(queryClient, () => {
            callCount += 1
        })

        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({ editorSemanticHighlighting: true }))
        expect(callCount).toBe(0)

        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({ editorSemanticHighlighting: false, keymapOverrides: 'unrelated-change' }))
        expect(callCount).toBe(1)

        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({ editorSemanticHighlighting: false, keymapOverrides: 'another-change' }))
        expect(callCount).toBe(1)

        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({ editorSemanticHighlighting: true }))
        expect(callCount).toBe(2)

        unsubscribe()
        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({ editorSemanticHighlighting: false }))
        expect(callCount).toBe(2)
    })

    test('editorSemanticHighlighting 이 없으면 true 로 취급한다 — 원래 로직의 ?? true 기본값과 동등', async () => {
        const { observeSemanticHighlightingSetting } = await importUseLspSession()
        const queryClient = new QueryClient()
        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({}))

        let callCount = 0
        observeSemanticHighlightingSetting(queryClient, () => {
            callCount += 1
        })

        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({ editorCodeLensEnabled: false }))
        expect(callCount).toBe(0)

        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settingsWith({ editorSemanticHighlighting: false }))
        expect(callCount).toBe(1)
    })

    test('옵저버 자체는 fetch 하지 않는다(enabled:false) — 캐시가 비어 있어도 요청을 만들지 않는다', async () => {
        const { observeSemanticHighlightingSetting } = await importUseLspSession()
        const queryClient = new QueryClient()

        observeSemanticHighlightingSetting(queryClient, () => {})
        await new Promise((resolve) => setTimeout(resolve, 0))

        const state = queryClient.getQueryState(QUERY_KEY.SETTINGS.CURRENT)
        expect(state?.fetchStatus).toBe('idle')
        expect(state?.dataUpdateCount).toBe(0)
    })
})
