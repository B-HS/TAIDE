import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, test } from 'bun:test'

const BUNDLED_THEMES_DIR = join(import.meta.dir, '../../../../src-tauri/resources/themes')
const THIRD_PARTY_LICENSES_PATH = join(import.meta.dir, '../../../../THIRD_PARTY_LICENSES.md')

/**
 * Matches the theme count THIRD_PARTY_LICENSES.md states in its "Bundled Themes" preamble. That
 * literal had gone stale (it still read 36 while 38 themes shipped — see
 * `docs/acknowledge/2026-09-15-d61-theme-state-distinctness-contract.md` §0) because adding a theme
 * only ever tripped the per-id gate below; nothing read the prose number.
 */
const BUNDLED_THEME_COUNT_PATTERN = /TAIDE ships (\d+) color themes/

const readBundledThemeIds = () =>
    readdirSync(BUNDLED_THEMES_DIR)
        .filter((name) => name.endsWith('.json'))
        .map((name) => name.replace(/\.json$/, ''))

describe('번들 테마 라이선스 등재 게이트', () => {
    test('src-tauri/resources/themes/*.json 전량의 id 가 루트 THIRD_PARTY_LICENSES.md 에 백틱 id 로 등장한다', () => {
        const ids = readBundledThemeIds()
        expect(ids.length).toBeGreaterThan(0)

        const licenses = readFileSync(THIRD_PARTY_LICENSES_PATH, 'utf-8')
        const missing = ids.filter((id) => !licenses.includes(`\`${id}\``))

        expect(missing).toEqual([])
    })

    test('THIRD_PARTY_LICENSES.md 가 명시한 번들 테마 개수가 실제 JSON 개수와 일치한다', () => {
        const licenses = readFileSync(THIRD_PARTY_LICENSES_PATH, 'utf-8')
        const declared = BUNDLED_THEME_COUNT_PATTERN.exec(licenses)

        expect(declared).not.toBeNull()
        expect(Number(declared?.[1])).toBe(readBundledThemeIds().length)
    })
})
