import type { LanguageRegistration } from '@shikijs/core'

export const TAIDE_LANGUAGE_IDS = [
    'rust',
    'typescript',
    'typescriptreact',
    'javascript',
    'javascriptreact',
    'json',
    'jsonc',
    'markdown',
    'toml',
    'yaml',
    'html',
    'css',
    'scss',
    'python',
    'go',
    'shellscript',
    'java',
    'ruby',
    'erb',
    'dart',
    'swift',
    'scala',
    'elixir',
    'heex',
    'haskell',
    'c',
    'cpp',
    'kotlin',
    'lua',
    'zig',
    'hcl',
] as const

export type TaideLanguageId = (typeof TAIDE_LANGUAGE_IDS)[number]

const TAIDE_LANGUAGE_ID_SET: ReadonlySet<string> = new Set(TAIDE_LANGUAGE_IDS)

export const isTaideLanguageId = (languageId: string): languageId is TaideLanguageId => TAIDE_LANGUAGE_ID_SET.has(languageId)

/**
 * The grammars loaded into the highlighter at boot; every other entry of {@link TAIDE_LANGUAGE_IDS}
 * is fetched on demand the first time a model of that language exists (see `shiki-monaco.ts`'s
 * `ensureShikiLanguage`). Loading all 31 up front cost 2266kB across 30 built grammar chunks —
 * `cpp` alone is 778kB — fetched and parsed on the main thread during startup for languages most
 * sessions never open (audit §1-7). These three total 64kB.
 *
 * The three kept here are the ones the app itself opens without any user action, so deferring them
 * would only trade a boot cost for a guaranteed re-tokenize a moment later: `settings.json` and
 * `keybindings.json` open as app-file tabs (`json`/`jsonc`), and markdown is what the welcome and
 * preview surfaces render. Anything the user opens — including restored session tabs — arrives
 * through the on-demand path.
 */
export const TAIDE_CORE_LANGUAGE_IDS = ['json', 'jsonc', 'markdown'] as const satisfies readonly TaideLanguageId[]

type GrammarLoader = () => Promise<LanguageRegistration[]>

const renameMainGrammar = (registrations: LanguageRegistration[], name: string): LanguageRegistration[] => {
    const mainIndex = registrations.length - 1
    return registrations.map((registration, index) => (index === mainIndex ? { ...registration, name } : registration))
}

const GRAMMAR_LOADERS: Record<TaideLanguageId, GrammarLoader> = {
    rust: async () => (await import('@shikijs/langs/rust')).default,
    typescript: async () => (await import('@shikijs/langs/typescript')).default,
    typescriptreact: async () => renameMainGrammar((await import('@shikijs/langs/tsx')).default, 'typescriptreact'),
    javascript: async () => (await import('@shikijs/langs/javascript')).default,
    javascriptreact: async () => renameMainGrammar((await import('@shikijs/langs/jsx')).default, 'javascriptreact'),
    json: async () => (await import('@shikijs/langs/json')).default,
    jsonc: async () => (await import('@shikijs/langs/jsonc')).default,
    markdown: async () => (await import('@shikijs/langs/markdown')).default,
    toml: async () => (await import('@shikijs/langs/toml')).default,
    yaml: async () => (await import('@shikijs/langs/yaml')).default,
    html: async () => (await import('@shikijs/langs/html')).default,
    css: async () => (await import('@shikijs/langs/css')).default,
    scss: async () => (await import('@shikijs/langs/scss')).default,
    python: async () => (await import('@shikijs/langs/python')).default,
    go: async () => (await import('@shikijs/langs/go')).default,
    shellscript: async () => (await import('@shikijs/langs/shellscript')).default,
    java: async () => (await import('@shikijs/langs/java')).default,
    ruby: async () => (await import('@shikijs/langs/ruby')).default,
    erb: async () => (await import('@shikijs/langs/erb')).default,
    dart: async () => (await import('@shikijs/langs/dart')).default,
    swift: async () => (await import('@shikijs/langs/swift')).default,
    scala: async () => (await import('@shikijs/langs/scala')).default,
    elixir: async () => (await import('@shikijs/langs/elixir')).default,
    heex: async () => renameMainGrammar((await import('@shikijs/langs/html')).default, 'heex'),
    haskell: async () => (await import('@shikijs/langs/haskell')).default,
    c: async () => (await import('@shikijs/langs/c')).default,
    cpp: async () => (await import('@shikijs/langs/cpp')).default,
    kotlin: async () => (await import('@shikijs/langs/kotlin')).default,
    lua: async () => (await import('@shikijs/langs/lua')).default,
    zig: async () => (await import('@shikijs/langs/zig')).default,
    hcl: async () => (await import('@shikijs/langs/hcl')).default,
}

export const loadTaideGrammar = (id: TaideLanguageId) => GRAMMAR_LOADERS[id]()

/**
 * Each loader's array is self-contained — shiki ships every grammar a language embeds alongside it,
 * so a per-language load never leaves an `embeddedLangs` name unresolved (which would make
 * `Registry.loadLanguages` throw). That is what lets the highlighter be built from an arbitrary
 * subset of {@link TAIDE_LANGUAGE_IDS} and grown one language at a time later.
 */
export const loadTaideGrammars = async (ids: readonly TaideLanguageId[]): Promise<LanguageRegistration[]> => {
    const grammarLists = await Promise.all(ids.map(loadTaideGrammar))
    return grammarLists.flat()
}
