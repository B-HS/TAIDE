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

export type ThemeDraft = {
    id: string
    name: string
    themeType: ThemeType
    extendsId: string
    base: ThemeValues
    current: ThemeValues
}

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
}): ThemeDraft => ({
    id: params.id,
    name: params.name,
    themeType: params.themeType,
    extendsId: params.extendsId,
    base: params.base,
    current: params.initial ?? { colors: { ...params.base.colors }, syntax: { ...params.base.syntax }, terminal: { ...params.base.terminal } },
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
    }
}

export const isThemeDraftValid = (draft: ThemeDraft) =>
    draft.name.trim().length > 0 &&
    Object.values(draft.current.colors).every(isValidThemeColorValue) &&
    Object.values(draft.current.terminal).every(isValidThemeColorValue) &&
    Object.values(draft.current.syntax).every((style) => isValidThemeColorValue(style.fg))
