export type TokensProviderRegistration = { dispose: () => void }

/**
 * The slice of `monaco.languages` this module drives. Declared with method shorthand so the real
 * namespace (whose `provider` parameter is a union of monaco's own provider types) stays assignable
 * to a signature that accepts `unknown` — the same bivariance opt-in
 * `LeadingTrailingDebouncerScheduler` documents.
 */
export type TokensProviderHost = {
    setTokensProvider(languageId: string, provider: never): TokensProviderRegistration
}

type SwapTokensProviderRegistrationsInput = {
    host: TokensProviderHost
    previous: readonly TokensProviderRegistration[]
    attach: () => void
}

/**
 * Runs `attach` (i.e. `shikiToMonaco`) while capturing every `setTokensProvider` registration it
 * makes, then disposes the registrations from the previous attach and returns the new set.
 *
 * `shikiToMonaco` registers one tokens provider per language the highlighter has loaded, each
 * closing over *that* highlighter, and returns nothing — so a rebuild with a smaller grammar set
 * (uninstalling a plugin that contributed a language) silently left the removed language's provider
 * registered against the highlighter `reinitShiki` was about to dispose. Monaco cannot unregister a
 * language, so the language id and its file associations stay; opening one of those files then
 * tokenized through `highlighter.getLanguage(...)` on a disposed instance, which throws
 * (`ShikiError: Shiki instance has been disposed`) for every line, forever (audit §4-B D6).
 *
 * Disposing *after* the new registration is deliberate and safe: monaco's `TokenizationRegistry`
 * dispose is identity-checked (`if (this._tokenizationSupports.get(languageId) !== support) return`),
 * so a language re-registered by this attach keeps its new provider and only the languages that
 * disappeared are actually deregistered — those fall back to monaco's plain tokenization instead of
 * a throwing one. If `attach` throws, the previous set is left in place (a half-built new set must
 * not replace a working one) and the host's method is still restored.
 */
export const swapTokensProviderRegistrations = ({ host, previous, attach }: SwapTokensProviderRegistrationsInput): TokensProviderRegistration[] => {
    const originalSetTokensProvider = host.setTokensProvider
    const registrations: TokensProviderRegistration[] = []

    host.setTokensProvider = (languageId, provider) => {
        const registration = originalSetTokensProvider.call(host, languageId, provider)
        registrations.push(registration)
        return registration
    }

    try {
        attach()
    } finally {
        host.setTokensProvider = originalSetTokensProvider
    }

    previous.forEach((registration) => registration.dispose())
    return registrations
}
