import { describe, expect, test } from 'bun:test'
import { RawFileReadError } from '@entities/file/file.raw'

describe('RawFileReadError', () => {
    test('Localized AppError 는 fallback 문자열을 message 로 노출한다 ("[object Object]" 방지 — root_guard 의 Forbidden 2건이 통과하는 경로)', () => {
        const error = new RawFileReadError({
            code: 'Localized',
            message: {
                kind: 'Forbidden',
                key: 'error.path.outsideOpenProjects',
                args: { path: '/etc/passwd' },
                fallback: 'Path is outside the open project roots: /etc/passwd',
            },
        })

        expect(error.message).toBe('Path is outside the open project roots: /etc/passwd')
        expect(error.code).toBe('Forbidden')
        expect(error.localeKey).toBe('error.path.outsideOpenProjects')
        expect(error.localeArgs).toEqual({ path: '/etc/passwd' })
    })

    test('비-Localized AppError 는 기존과 동일하게 message 를 그대로 노출한다', () => {
        const error = new RawFileReadError({ code: 'NotFound', message: 'no such file' })

        expect(error.message).toBe('no such file')
        expect(error.code).toBe('NotFound')
        expect(error.localeKey).toBeUndefined()
    })
})
