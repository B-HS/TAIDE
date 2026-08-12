import { describe, expect, test } from 'bun:test'
import type { LoadedPlugin, PluginLanguageContribution } from '@shared/api/bindings'
import { assemblePluginGrammarRegistrations, buildGrammarRegistration } from '@entities/plugin/plugin-grammar'

const contribution = (overrides: Partial<PluginLanguageContribution> = {}): PluginLanguageContribution => ({
    id: 'zig',
    extensions: ['.zig'],
    grammar: 'grammars/zig.tmLanguage.json',
    ...overrides,
})

const plugin = (overrides: Partial<LoadedPlugin> = {}): LoadedPlugin => ({
    manifest: {
        manifestVersion: 1,
        id: 'taide-plugin-zig',
        name: 'Zig',
        version: '1.0.0',
        contributes: { languages: [contribution()] },
    },
    root: '/plugins/taide-plugin-zig',
    enabled: true,
    ...overrides,
})

describe('buildGrammarRegistration', () => {
    test('scopeName·patterns·repository 를 갖춘 grammar 를 LanguageRegistration 으로 조립한다', () => {
        const grammarJson = JSON.stringify({ scopeName: 'source.zig', patterns: [{ match: 'fn' }], repository: { keywords: {} } })

        const registration = buildGrammarRegistration(contribution(), grammarJson)

        expect(registration?.name).toBe('zig')
        expect(registration?.scopeName).toBe('source.zig')
        expect(registration?.patterns).toEqual([{ match: 'fn' }])
        expect(registration?.repository).toEqual({ keywords: {} })
    })

    test('patterns·repository 가 없는 grammar 는 빈 값으로 채운다', () => {
        const registration = buildGrammarRegistration(contribution(), JSON.stringify({ scopeName: 'source.zig' }))

        expect(registration?.patterns).toEqual([])
        expect(registration?.repository).toEqual({})
    })

    test('embeddedLanguages 기여 필드를 embeddedLangs 로 매핑한다', () => {
        const registration = buildGrammarRegistration(
            contribution({ embeddedLanguages: ['html', 'ruby'] }),
            JSON.stringify({ scopeName: 'source.erb' }),
        )

        expect(registration?.embeddedLangs).toEqual(['html', 'ruby'])
    })

    test('JSON 파싱이 실패하면 null 을 반환한다', () => {
        expect(buildGrammarRegistration(contribution(), '{ not json')).toBeNull()
    })

    test('scopeName 이 없으면 null 을 반환한다', () => {
        expect(buildGrammarRegistration(contribution(), JSON.stringify({ patterns: [] }))).toBeNull()
    })

    test('scopeName 이 빈 문자열이면 null 을 반환한다', () => {
        expect(buildGrammarRegistration(contribution(), JSON.stringify({ scopeName: '' }))).toBeNull()
    })
})

describe('assemblePluginGrammarRegistrations', () => {
    test('활성 플러그인의 grammar 기여를 조립한다', async () => {
        const readGrammar = async (_pluginId: string, _languageId: string) => JSON.stringify({ scopeName: 'source.zig', patterns: [] })

        const registrations = await assemblePluginGrammarRegistrations([plugin()], readGrammar)

        expect(registrations).toHaveLength(1)
        expect(registrations[0].name).toBe('zig')
    })

    test('비활성 플러그인은 건너뛴다', async () => {
        const readGrammar = async () => JSON.stringify({ scopeName: 'source.zig' })

        const registrations = await assemblePluginGrammarRegistrations([plugin({ enabled: false })], readGrammar)

        expect(registrations).toEqual([])
    })

    test('grammar 가 없는 language 기여는 건너뛴다', async () => {
        const readGrammar = async () => JSON.stringify({ scopeName: 'source.zig' })
        const noGrammarPlugin = plugin({
            manifest: {
                manifestVersion: 1,
                id: 'taide-plugin-no-grammar',
                name: 'No Grammar',
                version: '1.0.0',
                contributes: { languages: [contribution({ grammar: null })] },
            },
        })

        const registrations = await assemblePluginGrammarRegistrations([noGrammarPlugin], readGrammar)

        expect(registrations).toEqual([])
    })

    test('IPC 조회가 실패한 항목은 조용히 건너뛰고 나머지는 조립한다', async () => {
        const twoLanguagePlugin = plugin({
            manifest: {
                manifestVersion: 1,
                id: 'taide-plugin-multi',
                name: 'Multi',
                version: '1.0.0',
                contributes: { languages: [contribution({ id: 'ok-lang' }), contribution({ id: 'broken-lang' })] },
            },
        })
        const readGrammar = async (_pluginId: string, languageId: string) => {
            if (languageId === 'broken-lang') throw new Error('grammar file not found')
            return JSON.stringify({ scopeName: 'source.ok' })
        }

        const registrations = await assemblePluginGrammarRegistrations([twoLanguagePlugin], readGrammar)

        expect(registrations).toHaveLength(1)
        expect(registrations[0].name).toBe('ok-lang')
    })

    test('개별 grammar 본문이 깨져도 나머지 등록은 그대로 반환한다', async () => {
        const twoLanguagePlugin = plugin({
            manifest: {
                manifestVersion: 1,
                id: 'taide-plugin-multi-invalid',
                name: 'Multi Invalid',
                version: '1.0.0',
                contributes: { languages: [contribution({ id: 'valid-lang' }), contribution({ id: 'invalid-lang' })] },
            },
        })
        const readGrammar = async (_pluginId: string, languageId: string) =>
            languageId === 'invalid-lang' ? '{ not json' : JSON.stringify({ scopeName: 'source.ok' })

        const registrations = await assemblePluginGrammarRegistrations([twoLanguagePlugin], readGrammar)

        expect(registrations).toHaveLength(1)
        expect(registrations[0].name).toBe('valid-lang')
    })
})
