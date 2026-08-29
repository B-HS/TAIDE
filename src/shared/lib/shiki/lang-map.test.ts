import { describe, expect, test } from 'bun:test'
import { isTaideLanguageId, loadTaideGrammar, loadTaideGrammars, TAIDE_CORE_LANGUAGE_IDS, TAIDE_LANGUAGE_IDS } from '@shared/lib/shiki/lang-map'

const EXPECTED_LANGUAGE_COUNT = 31

describe('TAIDE_LANGUAGE_IDS', () => {
    test('TAIDE 31개 언어 id 를 전부 포함하고 중복이 없다', () => {
        expect(TAIDE_LANGUAGE_IDS.length).toBe(EXPECTED_LANGUAGE_COUNT)
        expect(new Set(TAIDE_LANGUAGE_IDS).size).toBe(EXPECTED_LANGUAGE_COUNT)
    })

    test('plaintext 를 포함하지 않는다', () => {
        expect(TAIDE_LANGUAGE_IDS).not.toContain('plaintext')
    })
})

describe('TAIDE_CORE_LANGUAGE_IDS', () => {
    test('전체 목록의 진부분집합이라 나머지는 온디맨드 로드 대상으로 남는다', () => {
        expect(TAIDE_CORE_LANGUAGE_IDS.length).toBeLessThan(TAIDE_LANGUAGE_IDS.length)
        for (const id of TAIDE_CORE_LANGUAGE_IDS) expect(TAIDE_LANGUAGE_IDS).toContain(id)
    })

    test('부팅 시 가장 무거운 grammar(cpp·typescript 계열)는 코어에 들어있지 않다', () => {
        for (const id of ['cpp', 'typescript', 'typescriptreact', 'javascript', 'javascriptreact'] as const) {
            expect(TAIDE_CORE_LANGUAGE_IDS).not.toContain(id)
        }
    })
})

describe('isTaideLanguageId', () => {
    test('TAIDE 목록의 id 만 true 다', () => {
        expect(isTaideLanguageId('rust')).toBe(true)
        expect(isTaideLanguageId('typescriptreact')).toBe(true)
    })

    test('monaco 기본 언어·플러그인 언어처럼 목록 밖 id 는 false 다', () => {
        expect(isTaideLanguageId('plaintext')).toBe(false)
        expect(isTaideLanguageId('my-plugin-lang')).toBe(false)
        expect(isTaideLanguageId('')).toBe(false)
    })
})

describe('loadTaideGrammars', () => {
    test('요청한 id 들의 grammar 를 한 배열로 평탄화해 돌려준다', async () => {
        const registrations = await loadTaideGrammars(['json', 'markdown'])
        const names = registrations.map((registration) => registration.name)
        expect(names).toContain('json')
        expect(names).toContain('markdown')
    })

    test('빈 목록이면 빈 배열이다 (코어 0개 구성도 하이라이터를 만들 수 있다)', async () => {
        expect(await loadTaideGrammars([])).toEqual([])
    })

    test('id 하나의 grammar 묶음은 자기 embeddedLangs 를 스스로 해소한다 — 언어 단위 온디맨드 로드의 전제', async () => {
        for (const id of TAIDE_LANGUAGE_IDS) {
            const registrations = await loadTaideGrammar(id)
            const loadedNames = new Set(registrations.map((registration) => registration.name))
            const unresolved = registrations.flatMap((registration) => registration.embeddedLangs ?? []).filter((name) => !loadedNames.has(name))
            expect(unresolved).toEqual([])
        }
    })
})

describe('loadTaideGrammar', () => {
    test('전 언어 id 가 예외 없이 grammar 를 로드하고 마지막(main) 항목이 TAIDE id 로 등록된다', async () => {
        for (const id of TAIDE_LANGUAGE_IDS) {
            const registrations = await loadTaideGrammar(id)
            expect(registrations.length).toBeGreaterThan(0)
            expect(registrations[registrations.length - 1]?.name).toBe(id)
        }
    })

    test('typescriptreact 는 tsx grammar 를 재명명해 로드한다', async () => {
        const registrations = await loadTaideGrammar('typescriptreact')
        const main = registrations[registrations.length - 1]
        expect(main?.name).toBe('typescriptreact')
        expect(main?.scopeName).toBe('source.tsx')
    })

    test('javascriptreact 는 jsx grammar 를 재명명해 로드한다', async () => {
        const registrations = await loadTaideGrammar('javascriptreact')
        const main = registrations[registrations.length - 1]
        expect(main?.name).toBe('javascriptreact')
        expect(main?.scopeName).toBe('source.js.jsx')
    })

    test('heex 는 html grammar 를 폴백으로 재명명해 로드하고, scopeName 은 html 과 동일하다', async () => {
        const [htmlRegistrations, heexRegistrations] = await Promise.all([loadTaideGrammar('html'), loadTaideGrammar('heex')])
        const htmlMain = htmlRegistrations[htmlRegistrations.length - 1]
        const heexMain = heexRegistrations[heexRegistrations.length - 1]
        expect(htmlMain?.name).toBe('html')
        expect(heexMain?.name).toBe('heex')
        expect(heexMain?.scopeName).toBe(htmlMain?.scopeName)
    })
})
