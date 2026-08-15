import { describe, expect, test } from 'bun:test'
import { relativeTimeToken } from '@shared/lib/relative-time'

const SECONDS_PER_MINUTE = 60
const MINUTES_PER_HOUR = 60
const HOURS_PER_DAY = 24

const nowUnix = () => Math.floor(Date.now() / 1000)

describe('relativeTimeToken', () => {
    test('방금 전이면 justNow 키를 반환한다', () => {
        expect(relativeTimeToken(nowUnix())).toEqual({ key: 'git.timeJustNow', params: undefined })
    })

    test('분 단위 경과는 timeMinutesAgo 키와 n 을 반환한다', () => {
        expect(relativeTimeToken(nowUnix() - 5 * SECONDS_PER_MINUTE)).toEqual({ key: 'git.timeMinutesAgo', params: { n: 5 } })
    })

    test('시간 단위 경과는 timeHoursAgo 키와 n 을 반환한다', () => {
        expect(relativeTimeToken(nowUnix() - 3 * SECONDS_PER_MINUTE * MINUTES_PER_HOUR)).toEqual({ key: 'git.timeHoursAgo', params: { n: 3 } })
    })

    test('일 단위 경과는 timeDaysAgo 키와 n 을 반환한다', () => {
        expect(relativeTimeToken(nowUnix() - 2 * SECONDS_PER_MINUTE * MINUTES_PER_HOUR * HOURS_PER_DAY)).toEqual({
            key: 'git.timeDaysAgo',
            params: { n: 2 },
        })
    })

    test('미래 timeUnix 는 음수 경과를 0 으로 clamp 해 justNow 를 반환한다', () => {
        expect(relativeTimeToken(nowUnix() + 999)).toEqual({ key: 'git.timeJustNow', params: undefined })
    })
})
