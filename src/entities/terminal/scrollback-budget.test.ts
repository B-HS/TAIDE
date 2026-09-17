import { describe, expect, test } from 'bun:test'
import { DEFAULT_SCROLLBACK_BYTES, MAX_SCROLLBACK_BYTES, SCROLLBACK_BYTES_PER_LINE_ESTIMATE } from '@shared/constants/terminal'
import { resolveScrollbackBytes } from '@entities/terminal/scrollback-budget'

const MIN_SETTING_LINES = 100
const MAX_SETTING_LINES = 100_000
const MID_SETTING_LINES = 20_000

describe('resolveScrollbackBytes', () => {
    test('줄 수에 줄당 바이트 추정치를 곱한다 — 설정과 Rust 링 용량을 잇는 유일한 지점', () => {
        expect(resolveScrollbackBytes(MID_SETTING_LINES)).toBe(MID_SETTING_LINES * SCROLLBACK_BYTES_PER_LINE_ESTIMATE)
    })

    test('설정 최솟값(100줄)은 하한인 2MiB 로 올린다 — 예전 고정 예산보다 좁아지지 않는다', () => {
        expect(resolveScrollbackBytes(MIN_SETTING_LINES)).toBe(DEFAULT_SCROLLBACK_BYTES)
    })

    test('설정 최댓값(100,000줄)은 상한인 32MiB 로 자른다 — 링은 상주 메모리라 무한 요청을 받지 않는다', () => {
        expect(resolveScrollbackBytes(MAX_SETTING_LINES)).toBe(MAX_SCROLLBACK_BYTES)
    })

    test('설정이 아직 없으면(null·undefined) 예전 스폰과 같은 기본 예산을 쓴다', () => {
        expect(resolveScrollbackBytes(null)).toBe(DEFAULT_SCROLLBACK_BYTES)
        expect(resolveScrollbackBytes(undefined)).toBe(DEFAULT_SCROLLBACK_BYTES)
    })

    test('유한하지 않은 값은 기본 예산으로 떨어진다 — NaN 이 clamp 를 통과해 스폰을 깨뜨리지 않게', () => {
        expect(resolveScrollbackBytes(Number.NaN)).toBe(DEFAULT_SCROLLBACK_BYTES)
        expect(resolveScrollbackBytes(Number.POSITIVE_INFINITY)).toBe(DEFAULT_SCROLLBACK_BYTES)
    })

    test('Rust resolve_scrollback_bytes 의 clamp 범위와 같은 창을 쓴다', () => {
        expect(resolveScrollbackBytes(0)).toBe(DEFAULT_SCROLLBACK_BYTES)
        expect(resolveScrollbackBytes(Number.MAX_SAFE_INTEGER)).toBe(MAX_SCROLLBACK_BYTES)
    })
})
