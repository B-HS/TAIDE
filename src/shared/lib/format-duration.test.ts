import { describe, expect, test } from 'bun:test'
import { applyLocaleMessages, i18next } from '@shared/i18n/i18n'
import { formatDurationShort } from '@shared/lib/format-duration'

applyLocaleMessages(i18next.language, {
    'common.durationHoursMinutes': '{{hours}}h {{minutes}}m',
    'common.durationMinutesSeconds': '{{minutes}}m {{seconds}}s',
    'common.durationSeconds': '{{seconds}}s',
})

const SECOND_MS = 1_000
const MINUTE_MS = 60 * SECOND_MS
const HOUR_MS = 60 * MINUTE_MS

describe('formatDurationShort', () => {
    test('1분 미만은 초만 보여준다', () => {
        expect(formatDurationShort(45 * SECOND_MS)).toBe('45s')
    })

    test('1분 이상 1시간 미만은 분과 초를 함께 보여준다', () => {
        expect(formatDurationShort(3 * MINUTE_MS + 12 * SECOND_MS)).toBe('3m 12s')
    })

    test('1시간 이상은 시간과 분만 보여준다 (초는 잡음이라 버린다)', () => {
        expect(formatDurationShort(HOUR_MS + 2 * MINUTE_MS + 59 * SECOND_MS)).toBe('1h 2m')
    })

    test('나머지가 0이어도 자리를 유지한다', () => {
        expect(formatDurationShort(2 * MINUTE_MS)).toBe('2m 0s')
        expect(formatDurationShort(2 * HOUR_MS)).toBe('2h 0m')
    })

    test('초 미만은 0초로 내림한다 (밀리초 단위를 만들지 않는다)', () => {
        expect(formatDurationShort(999)).toBe('0s')
    })

    test('음수는 0초로 본다', () => {
        expect(formatDurationShort(-5_000)).toBe('0s')
    })
})
