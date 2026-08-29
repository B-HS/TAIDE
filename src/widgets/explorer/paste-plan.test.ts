import { describe, expect, test } from 'bun:test'
import { IpcError } from '@shared/api/unwrap-result'
import {
    DESTINATION_EXISTS_ERROR_KEY,
    type ExplorerClipboardEntry,
    isSamePlaceCutPaste,
    pasteWithUniqueEntryName,
} from '@widgets/explorer/paste-plan'

const destinationExistsError = () =>
    new IpcError({
        code: 'Localized',
        message: { kind: 'InvalidArgument', key: DESTINATION_EXISTS_ERROR_KEY, args: { path: '/p/a.ts' }, fallback: '이미 존재합니다' },
    })

const copyEntry: ExplorerClipboardEntry = { mode: 'copy', path: '/p/src/a.ts', kind: 'file' }

describe('isSamePlaceCutPaste', () => {
    test('잘라낸 항목을 원래 폴더에 붙여넣으면 no-op 으로 판정한다', () => {
        expect(isSamePlaceCutPaste({ mode: 'cut', path: '/p/src/a.ts', kind: 'file' }, '/p/src')).toBe(true)
    })

    test('잘라낸 항목을 다른 폴더에 붙여넣으면 실행 대상이다', () => {
        expect(isSamePlaceCutPaste({ mode: 'cut', path: '/p/src/a.ts', kind: 'file' }, '/p/lib')).toBe(false)
    })

    test('복사는 같은 폴더에 붙여넣어도 사본을 만든다', () => {
        expect(isSamePlaceCutPaste(copyEntry, '/p/src')).toBe(false)
    })
})

describe('pasteWithUniqueEntryName', () => {
    test('보이는 형제와 충돌하지 않으면 원래 이름 그대로 실행한다', async () => {
        const attempted: string[] = []
        const name = await pasteWithUniqueEntryName({
            clipboard: copyEntry,
            siblingNames: ['b.ts'],
            conflictSuffix: '복사본',
            run: async (destinationName) => {
                attempted.push(destinationName)
            },
        })

        expect(name).toBe('a.ts')
        expect(attempted).toEqual(['a.ts'])
    })

    test('접힌 폴더 때문에 보이지 않던 항목과 충돌하면 접미를 붙여 재시도한다', async () => {
        const onDisk = new Set(['a.ts', 'a 복사본.ts'])
        const attempted: string[] = []
        const name = await pasteWithUniqueEntryName({
            clipboard: copyEntry,
            siblingNames: [],
            conflictSuffix: '복사본',
            run: async (destinationName) => {
                attempted.push(destinationName)
                if (onDisk.has(destinationName)) throw destinationExistsError()
            },
        })

        expect(attempted).toEqual(['a.ts', 'a 복사본.ts', 'a 복사본 2.ts'])
        expect(name).toBe('a 복사본 2.ts')
    })

    test('폴더는 점 뒤를 확장자로 오분해하지 않는다', async () => {
        const attempted: string[] = []
        await pasteWithUniqueEntryName({
            clipboard: { mode: 'copy', path: '/p/src/v1.2', kind: 'directory' },
            siblingNames: ['v1.2'],
            conflictSuffix: '복사본',
            run: async (destinationName) => {
                attempted.push(destinationName)
            },
        })

        expect(attempted).toEqual(['v1.2 복사본'])
    })

    test('목적지 존재 외의 오류는 재시도 없이 그대로 전파한다', async () => {
        const attempted: string[] = []
        const failure = new Error('디스크 오류')

        await expect(
            pasteWithUniqueEntryName({
                clipboard: copyEntry,
                siblingNames: [],
                conflictSuffix: '복사본',
                run: async (destinationName) => {
                    attempted.push(destinationName)
                    throw failure
                },
            }),
        ).rejects.toThrow(failure)
        expect(attempted).toEqual(['a.ts'])
    })

    test('상한까지 모두 충돌하면 마지막 백엔드 오류를 그대로 던진다', async () => {
        const attempted: string[] = []

        await expect(
            pasteWithUniqueEntryName({
                clipboard: copyEntry,
                siblingNames: [],
                conflictSuffix: '복사본',
                attemptLimit: 3,
                run: async (destinationName) => {
                    attempted.push(destinationName)
                    throw destinationExistsError()
                },
            }),
        ).rejects.toBeInstanceOf(IpcError)
        expect(attempted).toEqual(['a.ts', 'a 복사본.ts', 'a 복사본 2.ts'])
    })
})
