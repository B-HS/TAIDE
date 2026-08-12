import type { LanguageRegistration } from '@shikijs/core'
import type { LoadedPlugin, PluginLanguageContribution } from '@shared/api/bindings'
import { listPlugins, readPluginGrammar } from '@entities/plugin/plugin.ipc'

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null

const hasNonEmptyScopeName = (value: Record<string, unknown>): value is Record<string, unknown> & { scopeName: string } =>
    typeof value.scopeName === 'string' && value.scopeName.length > 0

export const buildGrammarRegistration = (contribution: PluginLanguageContribution, grammarJson: string) => {
    let parsed: unknown
    try {
        parsed = JSON.parse(grammarJson)
    } catch {
        return null
    }
    if (!isRecord(parsed) || !hasNonEmptyScopeName(parsed)) return null

    const { patterns, repository, ...rest } = parsed
    const registration: LanguageRegistration = {
        ...rest,
        name: contribution.id,
        scopeName: parsed.scopeName,
        patterns: Array.isArray(patterns) ? patterns : [],
        repository: isRecord(repository) ? (repository as LanguageRegistration['repository']) : {},
        embeddedLangs: contribution.embeddedLanguages ?? undefined,
    }
    delete registration.embeddedLanguages
    return registration
}

export const assemblePluginGrammarRegistrations = async (plugins: LoadedPlugin[], readGrammar: typeof readPluginGrammar) => {
    const contributions = plugins
        .filter((plugin) => plugin.enabled)
        .flatMap((plugin) => (plugin.manifest.contributes?.languages ?? []).map((language) => ({ pluginId: plugin.manifest.id, language })))
        .filter(({ language }) => Boolean(language.grammar))

    const registrations = await Promise.all(
        contributions.map(async ({ pluginId, language }) => {
            try {
                const grammarJson = await readGrammar(pluginId, language.id)
                return buildGrammarRegistration(language, grammarJson)
            } catch {
                return null
            }
        }),
    )

    return registrations.filter((registration): registration is LanguageRegistration => registration !== null)
}

export const loadPluginGrammarRegistrations = async () => assemblePluginGrammarRegistrations(await listPlugins(), readPluginGrammar)
