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

export const loadAllTaideGrammars = async (): Promise<LanguageRegistration[]> => {
    const grammarLists = await Promise.all(TAIDE_LANGUAGE_IDS.map(loadTaideGrammar))
    return grammarLists.flat()
}
