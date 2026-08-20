import { extractFileExtension } from '@shared/lib/file-extension'
import type { TaideLanguageId } from '@shared/lib/shiki/lang-map'

/**
 * Extension-to-language mapping for git blob diffs (`commit-file-diff.tsx`), which — unlike an
 * open editor tab — has no backend-supplied `languageId` to read (`git_show_file` returns raw
 * blob text only). This is a hand-kept port of the Rust `language_id_for_path` table
 * (`src-tauri/src/domain/git/service.rs`); typing the values as {@link TaideLanguageId} at least
 * catches drift against the *other* copy of this ID set (`TAIDE_LANGUAGE_IDS`) at compile time —
 * a value here that no longer names a grammar `loadTaideGrammar` can load is a type error, not a
 * silent plaintext fallback discovered at runtime.
 */
const LANGUAGE_ID_BY_EXTENSION: Record<string, TaideLanguageId> = {
    rs: 'rust',
    ts: 'typescript',
    tsx: 'typescriptreact',
    js: 'javascript',
    jsx: 'javascriptreact',
    mjs: 'javascript',
    cjs: 'javascript',
    json: 'json',
    jsonc: 'jsonc',
    md: 'markdown',
    mdx: 'markdown',
    toml: 'toml',
    yaml: 'yaml',
    yml: 'yaml',
    html: 'html',
    css: 'css',
    scss: 'scss',
    py: 'python',
    go: 'go',
    sh: 'shellscript',
    bash: 'shellscript',
    java: 'java',
    rb: 'ruby',
    erb: 'erb',
    dart: 'dart',
    swift: 'swift',
    scala: 'scala',
    sbt: 'scala',
    ex: 'elixir',
    exs: 'elixir',
    heex: 'heex',
    hs: 'haskell',
    lhs: 'haskell',
    c: 'c',
    h: 'c',
    cpp: 'cpp',
    hpp: 'cpp',
    cc: 'cpp',
    kt: 'kotlin',
    kts: 'kotlin',
    lua: 'lua',
    zig: 'zig',
    tf: 'hcl',
}

const DEFAULT_LANGUAGE_ID = 'plaintext'

export const getLanguageIdFromPath = (path: string) => {
    const fileName = path.split('/').filter(Boolean).at(-1) ?? path
    const extension = extractFileExtension(fileName)
    if (!extension) return DEFAULT_LANGUAGE_ID
    return LANGUAGE_ID_BY_EXTENSION[extension] ?? DEFAULT_LANGUAGE_ID
}
