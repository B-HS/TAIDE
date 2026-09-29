import { afterEach, beforeEach, describe, expect, test } from 'bun:test'
import { spawnPty } from '@entities/terminal/terminal.ipc'
import { IpcError } from '@shared/api/unwrap-result'

const REJECTED_APP_ERROR = { code: 'NotFound', message: 'project not open: prj-1' }
const SPAWN_OPTIONS = { projectId: 'project-1', shell: '/bin/zsh', cwd: '/tmp', cols: 80, rows: 24 }
let originalInternals = window.__TAURI_INTERNALS__

beforeEach(() => {
    originalInternals = window.__TAURI_INTERNALS__
    window.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: () => Promise.reject(REJECTED_APP_ERROR),
    }
})

afterEach(() => {
    window.__TAURI_INTERNALS__ = originalInternals
})

describe('spawnPty', () => {
    test('raw invoke 가 AppError 로 거부하면 IpcError 로 정규화해 던진다 (terminal-session.tsx 의 describeIpcError 경로 복원)', async () => {
        const pending = spawnPty(SPAWN_OPTIONS, () => undefined)

        await expect(pending).rejects.toBeInstanceOf(IpcError)
    })

    test('정규화된 IpcError 는 code 와 message 를 보존한다', async () => {
        try {
            await spawnPty(SPAWN_OPTIONS, () => undefined)
            throw new Error('unreachable')
        } catch (error) {
            expect(error).toBeInstanceOf(IpcError)
            expect((error as InstanceType<typeof IpcError>).code).toBe('NotFound')
            expect((error as InstanceType<typeof IpcError>).message).toBe('project not open: prj-1')
        }
    })
})
