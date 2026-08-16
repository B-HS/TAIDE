import type { LoadedPlugin } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { TAIDE_LANGUAGE_IDS } from '@shared/lib/shiki/lang-map'

const TAIDE_LANGUAGE_ID_SET = new Set<string>(TAIDE_LANGUAGE_IDS)
const registeredPluginLanguageIds = new Set<string>()

/**
 * Registers every enabled plugin's `contributes.languages[]` ids with monaco's language service
 * (`monaco.languages.register`) — without this, a plugin language id monaco has never heard of
 * renders as `plaintext` even once its shiki grammar is loaded, because the editor never opens a
 * model under that language id in the first place (contract §3.4/C1). Idempotent across repeated
 * bootstrap/reload calls: an id already registered statically at bootstrap (`TAIDE_LANGUAGE_IDS`,
 * `shared/lib/monaco/setup.ts`) or by an earlier call here is skipped, so reloading/reinstalling
 * plugins never re-registers (and never duplicates) the same language id with monaco.
 */
export const registerPluginLanguages = (plugins: LoadedPlugin[]) => {
    for (const plugin of plugins) {
        if (!plugin.enabled) continue
        for (const language of plugin.manifest.contributes?.languages ?? []) {
            if (TAIDE_LANGUAGE_ID_SET.has(language.id) || registeredPluginLanguageIds.has(language.id)) continue
            monaco.languages.register({ id: language.id, extensions: language.extensions, aliases: language.aliases })
            registeredPluginLanguageIds.add(language.id)
        }
    }
}
