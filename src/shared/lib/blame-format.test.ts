import { describe, expect, test } from 'bun:test'
import { formatBlameLine, formatRelativeTime, truncateBlameMessage } from '@shared/lib/blame-format'

const NOW_MS = new Date('2026-08-06T00:00:00Z').getTime()

describe('formatRelativeTime', () => {
    test('timeUnix 이 null 이면 unknown time 을 반환한다', () => {
        expect(formatRelativeTime(null, NOW_MS)).toBe('unknown time')
    })

    test('1분 미만이면 just now 를 반환한다', () => {
        expect(formatRelativeTime(NOW_MS / 1000 - 30, NOW_MS)).toBe('just now')
    })

    test('분 단위 경과를 표기한다', () => {
        expect(formatRelativeTime(NOW_MS / 1000 - 5 * 60, NOW_MS)).toBe('5 minutes ago')
    })

    test('1분이면 단수형을 사용한다', () => {
        expect(formatRelativeTime(NOW_MS / 1000 - 60, NOW_MS)).toBe('1 minute ago')
    })

    test('시간 단위 경과를 표기한다', () => {
        expect(formatRelativeTime(NOW_MS / 1000 - 3 * 60 * 60, NOW_MS)).toBe('3 hours ago')
    })

    test('일 단위 경과를 표기한다', () => {
        expect(formatRelativeTime(NOW_MS / 1000 - 4 * 24 * 60 * 60, NOW_MS)).toBe('4 days ago')
    })

    test('개월 단위 경과를 표기한다', () => {
        expect(formatRelativeTime(NOW_MS / 1000 - 60 * 24 * 60 * 60, NOW_MS)).toBe('2 months ago')
    })

    test('년 단위 경과를 표기한다', () => {
        expect(formatRelativeTime(NOW_MS / 1000 - 400 * 24 * 60 * 60, NOW_MS)).toBe('1 year ago')
    })

    test('미래 시각은 음수 대신 0 으로 클램프한다', () => {
        expect(formatRelativeTime(NOW_MS / 1000 + 60, NOW_MS)).toBe('just now')
    })
})

describe('truncateBlameMessage', () => {
    test('50자 이하는 그대로 반환한다', () => {
        expect(truncateBlameMessage('fix: short message')).toBe('fix: short message')
    })

    test('50자 초과는 잘라내고 ... 을 붙인다', () => {
        const long = 'a'.repeat(60)
        const result = truncateBlameMessage(long)
        expect(result).toBe(`${'a'.repeat(50)}...`)
    })

    test('개행을 공백으로 정규화한다', () => {
        expect(truncateBlameMessage('fix:\nmultiline   message')).toBe('fix: multiline message')
    })
})

describe('formatBlameLine', () => {
    test('미커밋 라인은 고정 문구를 반환한다', () => {
        expect(formatBlameLine({ line: 1, commitId: '', author: '', timeUnix: null, summary: '', isUncommitted: true }, NOW_MS)).toBe(
            'You, now - Uncommitted changes',
        )
    })

    test('커밋된 라인은 author, 상대시각, 메시지를 조합한다', () => {
        const result = formatBlameLine(
            {
                line: 10,
                commitId: 'abc123',
                author: 'HS',
                timeUnix: NOW_MS / 1000 - 4 * 24 * 60 * 60,
                summary: 'fix: something broken',
                isUncommitted: false,
            },
            NOW_MS,
        )
        expect(result).toBe('HS, 4 days ago • fix: something broken')
    })

    test('긴 메시지는 절단된다', () => {
        const result = formatBlameLine(
            {
                line: 1,
                commitId: 'abc',
                author: 'HS',
                timeUnix: NOW_MS / 1000,
                summary: 'a'.repeat(60),
                isUncommitted: false,
            },
            NOW_MS,
        )
        expect(result).toBe(`HS, just now • ${'a'.repeat(50)}...`)
    })

    test('author 가 현재 사용자와 같으면 You 로 치환한다', () => {
        const result = formatBlameLine(
            { line: 1, commitId: 'abc', author: 'HS', timeUnix: NOW_MS / 1000, summary: 'fix: something', isUncommitted: false },
            NOW_MS,
            'HS',
        )
        expect(result).toBe('You, just now • fix: something')
    })

    test('author 가 현재 사용자와 다르면 그대로 표기한다', () => {
        const result = formatBlameLine(
            { line: 1, commitId: 'abc', author: 'Other', timeUnix: NOW_MS / 1000, summary: 'fix: something', isUncommitted: false },
            NOW_MS,
            'HS',
        )
        expect(result).toBe('Other, just now • fix: something')
    })

    test('현재 사용자 정보가 없으면 author 를 그대로 표기한다', () => {
        const result = formatBlameLine(
            { line: 1, commitId: 'abc', author: 'HS', timeUnix: NOW_MS / 1000, summary: 'fix: something', isUncommitted: false },
            NOW_MS,
            null,
        )
        expect(result).toBe('HS, just now • fix: something')
    })
})
