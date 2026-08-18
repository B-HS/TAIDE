import { describe, expect, test } from 'bun:test'
import type { StatusRow } from '@shared/api/bindings'
import { isPathConflicted } from '@widgets/editor-pane/conflict-status'

const statusRow = (overrides: Partial<StatusRow>): StatusRow => ({
    path: 'a.txt',
    absPath: '/repo/a.txt',
    isConflicted: false,
    ...overrides,
})

describe('isPathConflicted', () => {
    test('탭의 절대경로가 충돌 중인 row 의 absPath 와 일치하면 true 를 반환한다', () => {
        const rows = [statusRow({ path: 'a.txt', absPath: '/repo/a.txt', isConflicted: true })]
        expect(isPathConflicted(rows, '/repo/a.txt')).toBe(true)
    })

    test('repo-relative path 가 절대경로와 우연히 같은 문자열이어도 절대경로 기준으로만 매칭한다', () => {
        const rows = [statusRow({ path: '/repo/a.txt', absPath: '/repo/a.txt', isConflicted: true })]
        expect(isPathConflicted(rows, 'a.txt')).toBe(false)
    })

    test('충돌 중이 아닌 row 는 absPath 가 일치해도 false 를 반환한다', () => {
        const rows = [statusRow({ path: 'a.txt', absPath: '/repo/a.txt', isConflicted: false })]
        expect(isPathConflicted(rows, '/repo/a.txt')).toBe(false)
    })

    test('일치하는 row 가 없으면 false 를 반환한다', () => {
        const rows = [statusRow({ path: 'b.txt', absPath: '/repo/b.txt', isConflicted: true })]
        expect(isPathConflicted(rows, '/repo/a.txt')).toBe(false)
    })
})
