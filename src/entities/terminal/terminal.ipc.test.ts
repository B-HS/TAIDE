import { describe, expect, mock, test } from 'bun:test'
import * as tauriCore from '@tauri-apps/api/core'

class FakeChannel {
    onmessage: ((value: unknown) => void) | undefined
}

const invokeMock = mock(() => Promise.reject({ code: 'NotFound' as const, message: 'project not open: prj-1' }))

mock.module('@tauri-apps/api/core', () => ({ ...tauriCore, invoke: invokeMock, Channel: FakeChannel }))

const importTerminalIpc = () => import('@entities/terminal/terminal.ipc')
const importIpcError = () => import('@shared/api/unwrap-result')

describe('spawnPty', () => {
    test('raw invoke 가 AppError 로 거부하면 IpcError 로 정규화해 던진다 (terminal-session.tsx 의 describeIpcError 경로 복원)', async () => {
        const { spawnPty } = await importTerminalIpc()
        const { IpcError } = await importIpcError()

        const pending = spawnPty({ shell: '/bin/zsh', cwd: '/tmp', cols: 80, rows: 24 } as never, () => undefined)

        await expect(pending).rejects.toBeInstanceOf(IpcError)
    })

    test('정규화된 IpcError 는 code 와 message 를 보존한다', async () => {
        const { spawnPty } = await importTerminalIpc()
        const { IpcError } = await importIpcError()

        try {
            await spawnPty({ shell: '/bin/zsh', cwd: '/tmp', cols: 80, rows: 24 } as never, () => undefined)
            throw new Error('unreachable')
        } catch (error) {
            expect(error).toBeInstanceOf(IpcError)
            expect((error as InstanceType<typeof IpcError>).code).toBe('NotFound')
            expect((error as InstanceType<typeof IpcError>).message).toBe('project not open: prj-1')
        }
    })
})
