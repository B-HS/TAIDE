import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, test } from 'bun:test'

const BUNDLED_THEMES_DIR = join(import.meta.dir, '../../../../src-tauri/resources/themes')
const THIRD_PARTY_LICENSES_PATH = join(import.meta.dir, '../../../../THIRD_PARTY_LICENSES.md')

describe('번들 테마 라이선스 등재 게이트', () => {
    test('src-tauri/resources/themes/*.json 전량의 id 가 루트 THIRD_PARTY_LICENSES.md 에 백틱 id 로 등장한다', () => {
        const ids = readdirSync(BUNDLED_THEMES_DIR)
            .filter((name) => name.endsWith('.json'))
            .map((name) => name.replace(/\.json$/, ''))
        expect(ids.length).toBeGreaterThan(0)

        const licenses = readFileSync(THIRD_PARTY_LICENSES_PATH, 'utf-8')
        const missing = ids.filter((id) => !licenses.includes(`\`${id}\``))

        expect(missing).toEqual([])
    })
})
