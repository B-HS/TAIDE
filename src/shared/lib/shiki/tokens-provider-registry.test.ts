import { describe, expect, test } from 'bun:test'
import { swapTokensProviderRegistrations } from '@shared/lib/shiki/tokens-provider-registry'
import type { TokensProviderHost } from '@shared/lib/shiki/tokens-provider-registry'

type FakeRegistry = {
    host: TokensProviderHost
    providersByLanguageId: Map<string, object>
}

/**
 * Mirrors monaco's `TokenizationRegistry`: registering replaces the language's provider, and the
 * returned disposable only deregisters when it is still the registered one (identity check).
 */
const createFakeRegistry = (): FakeRegistry => {
    const providersByLanguageId = new Map<string, object>()
    return {
        providersByLanguageId,
        host: {
            setTokensProvider: (languageId, provider) => {
                const registered = provider as object
                providersByLanguageId.set(languageId, registered)
                return {
                    dispose: () => {
                        if (providersByLanguageId.get(languageId) !== registered) return
                        providersByLanguageId.delete(languageId)
                    },
                }
            },
        },
    }
}

const attachLanguages = (host: TokensProviderHost, languageIds: string[], marker: string) => () =>
    languageIds.forEach((languageId) => host.setTokensProvider(languageId, { marker, languageId } as never))

describe('swapTokensProviderRegistrations — 플러그인 제거 후 disposed highlighter 참조 (audit §4-B D6)', () => {
    test('사라진 언어의 이전 provider 는 해제되고, 남은 언어는 새 provider 로 교체된다', () => {
        const { host, providersByLanguageId } = createFakeRegistry()

        const first = swapTokensProviderRegistrations({ host, previous: [], attach: attachLanguages(host, ['ts', 'plugin-lang'], 'old') })
        expect([...providersByLanguageId.keys()].toSorted()).toEqual(['plugin-lang', 'ts'])

        swapTokensProviderRegistrations({ host, previous: first, attach: attachLanguages(host, ['ts'], 'new') })

        expect([...providersByLanguageId.keys()]).toEqual(['ts'])
        expect(providersByLanguageId.get('ts')).toMatchObject({ marker: 'new' })
    })

    test('attach 가 끝나면 host 의 setTokensProvider 원본이 복구된다', () => {
        const { host } = createFakeRegistry()
        const original = host.setTokensProvider

        swapTokensProviderRegistrations({ host, previous: [], attach: attachLanguages(host, ['ts'], 'only') })

        expect(host.setTokensProvider).toBe(original)
    })

    test('attach 가 던지면 원본을 복구하고 이전 등록을 남긴 채 전파한다', () => {
        const { host, providersByLanguageId } = createFakeRegistry()
        const original = host.setTokensProvider
        const previous = swapTokensProviderRegistrations({ host, previous: [], attach: attachLanguages(host, ['ts'], 'old') })

        expect(() =>
            swapTokensProviderRegistrations({
                host,
                previous,
                attach: () => {
                    throw new Error('grammar load failed')
                },
            }),
        ).toThrow('grammar load failed')

        expect(host.setTokensProvider).toBe(original)
        expect(providersByLanguageId.get('ts')).toMatchObject({ marker: 'old' })
    })
})
