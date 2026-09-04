import { afterEach, beforeEach, describe, expect, mock, test } from 'bun:test'
import * as tauriCore from '@tauri-apps/api/core'
import { applyNativePerfGate, isPerfEnabled } from '@shared/lib/perf-mark'
import { REMOTE_WINDOW_LABEL } from '@shared/lib/remote/tauri-internals-shim'

const invokedCommands: string[] = []

let respond: () => Promise<unknown> = () => Promise.resolve({ enabled: true, entries: [], counters: [] })

const invokeMock = mock((command: string) => {
    invokedCommands.push(command)
    return respond()
})

/**
 * `bindings.ts` is generated and calls `invoke` from `@tauri-apps/api/core` directly, so the fake
 * goes one level below it — replacing the generated surface itself would leave every other test
 * file in this process (mock.module is process-global) without the commands it imports. The rest of
 * the module is spread through unchanged for the same reason.
 */
mock.module('@tauri-apps/api/core', () => ({ ...tauriCore, invoke: invokeMock }))

const importPerfIpc = () => import('@entities/app/perf.ipc')

const setWindowLabel = (label: string) => {
    window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label } } }
}

beforeEach(() => {
    invokedCommands.splice(0)
    respond = () => Promise.resolve({ enabled: true, entries: [], counters: [] })
    applyNativePerfGate(false)
})

afterEach(() => {
    window.__TAURI_INTERNALS__ = undefined
})

describe('syncNativePerfGate', () => {
    test('데스크톱 창에서는 perf_snapshot 의 enabled 를 프론트 게이트에 반영한다', async () => {
        setWindowLabel('main')
        const { syncNativePerfGate } = await importPerfIpc()

        await syncNativePerfGate()

        expect(invokedCommands).toEqual(['perf_snapshot'])
        expect(isPerfEnabled()).toBe(true)
    })

    test('네이티브 게이트가 꺼져 있으면 프론트 계측도 끈다 — TAIDE_PERF=0 대조 실행', async () => {
        setWindowLabel('editor-1')
        applyNativePerfGate(true)
        respond = () => Promise.resolve({ enabled: false, entries: [], counters: [] })
        const { syncNativePerfGate } = await importPerfIpc()

        await syncNativePerfGate()

        expect(isPerfEnabled()).toBe(false)
    })

    test('원격 미러에서는 IPC 를 호출하지 않는다 — REMOTE_DENIED 정책', async () => {
        setWindowLabel(REMOTE_WINDOW_LABEL)
        const { syncNativePerfGate } = await importPerfIpc()

        await syncNativePerfGate()

        expect(invokedCommands).toEqual([])
        expect(isPerfEnabled()).toBe(false)
    })

    test('조회가 실패해도 던지지 않고 빌드 기본값을 유지한다', async () => {
        setWindowLabel('main')
        applyNativePerfGate(true)
        respond = () => Promise.reject({ code: 'Forbidden' as const, message: 'denied' })
        const { syncNativePerfGate } = await importPerfIpc()

        await syncNativePerfGate()

        expect(invokedCommands).toEqual(['perf_snapshot'])
        expect(isPerfEnabled()).toBe(true)
    })
})
