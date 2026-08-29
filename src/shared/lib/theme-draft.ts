import type { ResolvedTheme, SyntaxStyle, Theme, ThemeType } from '@shared/api/bindings'
import { isValidThemeColorValue } from '@shared/lib/color'

const THEME_SCHEMA_VERSION = 1
const THEME_ID_RANDOM_SUFFIX_RADIX = 36
const THEME_ID_RANDOM_SUFFIX_START = 2
const THEME_ID_RANDOM_SUFFIX_LENGTH = 6

export type ThemeValues = {
    colors: Record<string, string>
    syntax: Record<string, SyntaxStyle>
    terminal: Record<string, string>
}

/**
 * The parts of a theme the token editor never shows and therefore cannot reconstruct: the raw
 * TextMate `tokenColors` rules (what actually colors code for an imported VS Code theme — the
 * per-token `syntax` map the editor edits is only a coarse projection of them) plus the attribution
 * fields. Carried verbatim through the draft round trip because {@link buildThemeFromDraft} writes
 * the *whole* theme file: anything not on the draft is not written, and for a `.vsix`-imported
 * theme that meant one no-op Save permanently flattened its highlighting to the syntax-derived
 * fallback and erased its author/source (audit §4-B B6).
 */
export type ThemeDraftMetadata = Pick<ResolvedTheme, 'tokenColors' | 'author' | 'license' | 'source'>

export type ThemeDraft = {
    id: string
    name: string
    themeType: ThemeType
    extendsId: string
    base: ThemeValues
    current: ThemeValues
    metadata: ThemeDraftMetadata
}

const EMPTY_THEME_DRAFT_METADATA: ThemeDraftMetadata = { tokenColors: null, author: null, license: null, source: null }

const serializeTokenColors = (rules: ThemeDraftMetadata['tokenColors']) => JSON.stringify(rules ?? null)

/**
 * Decides which metadata a draft must carry *itself* rather than let the saved theme inherit from
 * `extends`. `tokenColors` resolve through the base theme when the theme has none of its own
 * (`domain/theme/service.rs`'s `resolve_token_colors`), so a `ResolvedTheme` cannot say on its own
 * whether the rules it reports are the theme's or the base's — comparing against the base's resolved
 * rules is what separates them. Dropping rules identical to the base keeps a duplicated bundled
 * theme inheriting (it stays in sync if the bundled theme's rules ever change), while a theme whose
 * rules differ — every `.vsix` import, since the builtin bases carry none — writes its own copy and
 * survives the round trip. Attribution is always carried: it describes where the theme came from,
 * which stays true for a derived copy.
 */
export const resolveThemeDraftMetadata = (source: ThemeDraftMetadata, base: Pick<ResolvedTheme, 'tokenColors'>): ThemeDraftMetadata => ({
    tokenColors: serializeTokenColors(source.tokenColors) === serializeTokenColors(base.tokenColors) ? null : (source.tokenColors ?? null),
    author: source.author ?? null,
    license: source.license ?? null,
    source: source.source ?? null,
})

export const toThemeValues = (resolved: Pick<ResolvedTheme, 'colors' | 'syntax' | 'terminal'>): ThemeValues => ({
    colors: { ...resolved.colors },
    syntax: { ...resolved.syntax },
    terminal: { ...resolved.terminal },
})

export const slugifyThemeId = (name: string) =>
    name
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-+|-+$/g, '')

export const generateUniqueThemeId = (name: string, existingIds: readonly string[]) => {
    const base = slugifyThemeId(name) || 'custom-theme'
    if (!existingIds.includes(base)) return base
    const suffix = Math.random().toString(THEME_ID_RANDOM_SUFFIX_RADIX).slice(THEME_ID_RANDOM_SUFFIX_START, THEME_ID_RANDOM_SUFFIX_LENGTH)
    return `${base}-${suffix}`
}

export const createThemeDraft = (params: {
    id: string
    name: string
    themeType: ThemeType
    extendsId: string
    base: ThemeValues
    initial?: ThemeValues
    metadata?: ThemeDraftMetadata
}): ThemeDraft => ({
    id: params.id,
    name: params.name,
    themeType: params.themeType,
    extendsId: params.extendsId,
    base: params.base,
    current: params.initial ?? { colors: { ...params.base.colors }, syntax: { ...params.base.syntax }, terminal: { ...params.base.terminal } },
    metadata: params.metadata ?? EMPTY_THEME_DRAFT_METADATA,
})

export const renameThemeDraft = (draft: ThemeDraft, name: string): ThemeDraft => ({ ...draft, name })

export const setColorToken = (draft: ThemeDraft, key: string, value: string): ThemeDraft => ({
    ...draft,
    current: { ...draft.current, colors: { ...draft.current.colors, [key]: value } },
})

export const setTerminalToken = (draft: ThemeDraft, key: string, value: string): ThemeDraft => ({
    ...draft,
    current: { ...draft.current, terminal: { ...draft.current.terminal, [key]: value } },
})

export const setSyntaxToken = (draft: ThemeDraft, key: string, patch: Partial<SyntaxStyle>): ThemeDraft => ({
    ...draft,
    current: {
        ...draft.current,
        syntax: {
            ...draft.current.syntax,
            [key]: { ...(draft.current.syntax[key] ?? draft.base.syntax[key]), ...patch },
        },
    },
})

export const resetColorToken = (draft: ThemeDraft, key: string): ThemeDraft => setColorToken(draft, key, draft.base.colors[key])

export const resetTerminalToken = (draft: ThemeDraft, key: string): ThemeDraft => setTerminalToken(draft, key, draft.base.terminal[key])

export const resetSyntaxToken = (draft: ThemeDraft, key: string): ThemeDraft => ({
    ...draft,
    current: { ...draft.current, syntax: { ...draft.current.syntax, [key]: draft.base.syntax[key] } },
})

export const isSyntaxStyleEqual = (a: SyntaxStyle | undefined, b: SyntaxStyle | undefined) =>
    a?.fg === b?.fg && Boolean(a?.bold) === Boolean(b?.bold) && Boolean(a?.italic) === Boolean(b?.italic)

export const isColorTokenChanged = (draft: ThemeDraft, key: string) => draft.current.colors[key] !== draft.base.colors[key]

export const isTerminalTokenChanged = (draft: ThemeDraft, key: string) => draft.current.terminal[key] !== draft.base.terminal[key]

export const isSyntaxTokenChanged = (draft: ThemeDraft, key: string) => !isSyntaxStyleEqual(draft.current.syntax[key], draft.base.syntax[key])

export const diffThemeValues = (base: ThemeValues, current: ThemeValues): ThemeValues => {
    const colors = Object.fromEntries(Object.entries(current.colors).filter(([key, value]) => value !== base.colors[key]))
    const terminal = Object.fromEntries(Object.entries(current.terminal).filter(([key, value]) => value !== base.terminal[key]))
    const syntax = Object.fromEntries(Object.entries(current.syntax).filter(([key, value]) => !isSyntaxStyleEqual(value, base.syntax[key])))
    return { colors, syntax, terminal }
}

export const countChangedTokens = (draft: ThemeDraft) => {
    const diff = diffThemeValues(draft.base, draft.current)
    return Object.keys(diff.colors).length + Object.keys(diff.syntax).length + Object.keys(diff.terminal).length
}

export const buildThemeFromDraft = (draft: ThemeDraft): Theme => {
    const diff = diffThemeValues(draft.base, draft.current)
    return {
        version: THEME_SCHEMA_VERSION,
        id: draft.id,
        name: draft.name,
        type: draft.themeType,
        extends: draft.extendsId,
        palette: {},
        colors: diff.colors,
        syntax: diff.syntax,
        terminal: diff.terminal,
        tokenColors: draft.metadata.tokenColors ?? null,
        author: draft.metadata.author ?? null,
        license: draft.metadata.license ?? null,
        source: draft.metadata.source ?? null,
    }
}

/**
 * Signature of everything the theme editor lets a user change, for comparing a draft against the
 * state it was loaded in. The editor discarded an edited draft on close with no prompt — and the
 * live preview means the window is *already showing* those edits, so nothing on screen says they
 * were never written (audit §4-B D6). Metadata is excluded: it is carried, never edited.
 */
export const serializeThemeDraftEdits = (draft: ThemeDraft) => JSON.stringify([draft.name, draft.current])

export const isThemeDraftValid = (draft: ThemeDraft) =>
    draft.name.trim().length > 0 &&
    Object.values(draft.current.colors).every(isValidThemeColorValue) &&
    Object.values(draft.current.terminal).every(isValidThemeColorValue) &&
    Object.values(draft.current.syntax).every((style) => isValidThemeColorValue(style.fg))
