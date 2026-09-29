import { afterEach, beforeEach, describe, expect, spyOn, test } from 'bun:test'
import { syncNativePerfGate } from '@entities/app/perf.ipc'
import { commands } from '@shared/api/bindings'
import { applyNativePerfGate, isPerfEnabled } from '@shared/lib/perf-mark'
import { REMOTE_WINDOW_LABEL } from '@shared/lib/remote/tauri-internals-shim'

const setWindowLabel = (label: string) => {
    window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label } } }
}

beforeEach(() => {
    applyNativePerfGate(false)
})

afterEach(() => {
    window.__TAURI_INTERNALS__ = undefined
})

describe('syncNativePerfGate', () => {
    test('데스크톱 창에서는 perf_snapshot 의 enabled 를 프론트 게이트에 반영한다', async () => {
        setWindowLabel('main')
        const perfSnapshot = spyOn(commands, 'perfSnapshot').mockResolvedValue({ status: 'ok', data: { enabled: true, entries: [], counters: [] } })

        await syncNativePerfGate()

        expect(perfSnapshot).toHaveBeenCalledTimes(1)
        expect(isPerfEnabled()).toBe(true)
    })

    test('네이티브 게이트가 꺼져 있으면 프론트 계측도 끈다 — TAIDE_PERF=0 대조 실행', async () => {
        setWindowLabel('editor-1')
        applyNativePerfGate(true)
        const perfSnapshot = spyOn(commands, 'perfSnapshot').mockResolvedValue({ status: 'ok', data: { enabled: false, entries: [], counters: [] } })

        await syncNativePerfGate()

        expect(perfSnapshot).toHaveBeenCalledTimes(1)
        expect(isPerfEnabled()).toBe(false)
    })

    test('원격 미러에서는 IPC 를 호출하지 않는다 — REMOTE_DENIED 정책', async () => {
        setWindowLabel(REMOTE_WINDOW_LABEL)
        const perfSnapshot = spyOn(commands, 'perfSnapshot')

        await syncNativePerfGate()

        expect(perfSnapshot).not.toHaveBeenCalled()
        expect(isPerfEnabled()).toBe(false)
    })

    test('조회가 실패해도 던지지 않고 빌드 기본값을 유지한다', async () => {
        setWindowLabel('main')
        applyNativePerfGate(true)
        const perfSnapshot = spyOn(commands, 'perfSnapshot').mockResolvedValue({
            status: 'error',
            error: { code: 'Forbidden', message: 'denied' },
        })

        await syncNativePerfGate()

        expect(perfSnapshot).toHaveBeenCalledTimes(1)
        expect(isPerfEnabled()).toBe(true)
    })
})
