import { describe, expect, test } from 'bun:test'
import { loadTaideGrammar, TAIDE_LANGUAGE_IDS } from '@shared/lib/shiki/lang-map'

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
