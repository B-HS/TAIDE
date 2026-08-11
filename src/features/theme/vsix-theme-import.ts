import type { Theme, ThemeType, VsixExtensionInfo, VsixExtractedTheme, VsixThemeExtractionResult } from '@shared/api/bindings'
import { convertVscodeTheme } from '@shared/lib/theme-convert/convert'
import { parseJsonc } from '@shared/lib/theme-convert/jsonc'
import { slugifyThemeId } from '@shared/lib/theme-draft'

const THEME_SCHEMA_VERSION = 1
const FALLBACK_THEME_ID_SLUG = 'imported-theme'
const FALLBACK_THEME_LABEL_SLUG = 'theme'
const DEDUPE_SUFFIX_START = 2

const UI_THEME_TYPE: Record<string, ThemeType> = {
    vs: 'light',
    'vs-dark': 'dark',
    'hc-black': 'dark',
    'hc-light': 'light',
}

const inferThemeType = (uiTheme: string): ThemeType => UI_THEME_TYPE[uiTheme] ?? 'dark'

export type VsixThemeImportFailureReason = 'parse' | 'incomplete' | 'contrast'

export type VsixThemeCandidate = {
    key: string
    label: string
    id: string
    themeType: ThemeType
    idCollides: boolean
    warningCount: number
    theme: Theme | null
    failureReason: VsixThemeImportFailureReason | null
}

const buildRawChain = (extracted: VsixExtractedTheme): Record<string, unknown>[] | null => {
    try {
        const baseChain = [...extracted.includeChain].reverse().map((entry) => parseJsonc(entry.rawJson))
        return [...baseChain, parseJsonc(extracted.rawJson)]
    } catch {
        return null
    }
}

const buildExtensionSlug = (extension: VsixExtensionInfo) => {
    const stableIdentity = [extension.publisher, extension.name].filter(Boolean).join('-')
    return slugifyThemeId(stableIdentity) || slugifyThemeId(extension.displayName)
}

const buildCandidateId = (extension: VsixExtensionInfo, label: string, isMultiple: boolean, usedIds: Set<string>) => {
    const base = buildExtensionSlug(extension) || FALLBACK_THEME_ID_SLUG
    const withLabel = isMultiple ? `${base}-${slugifyThemeId(label) || FALLBACK_THEME_LABEL_SLUG}` : base
    if (!usedIds.has(withLabel)) return withLabel

    let suffix = DEDUPE_SUFFIX_START
    while (usedIds.has(`${withLabel}-${suffix}`)) suffix += 1
    return `${withLabel}-${suffix}`
}

type ConversionResult = ReturnType<typeof convertVscodeTheme>

const resolveFailureReason = (
    rawChain: Record<string, unknown>[] | null,
    conversion: ConversionResult | null,
): VsixThemeImportFailureReason | null => {
    if (!rawChain || !conversion) return 'parse'
    if (conversion.status !== 'ok') return 'incomplete'
    if (conversion.outputColorErrors.length > 0) return 'contrast'
    return null
}

const buildTheme = (id: string, label: string, themeType: ThemeType, conversion: ConversionResult, extension: VsixExtensionInfo): Theme => ({
    version: THEME_SCHEMA_VERSION,
    id,
    name: label,
    type: themeType,
    palette: {},
    colors: conversion.colors,
    syntax: conversion.syntax,
    terminal: conversion.terminal,
    author: extension.publisher || null,
    license: null,
    source: [extension.displayName, extension.version].filter(Boolean).join(' ') || null,
})

export const buildVsixThemeCandidates = (result: VsixThemeExtractionResult, existingThemeIds: readonly string[]): VsixThemeCandidate[] => {
    const usedIds = new Set<string>()
    const isMultiple = result.themes.length > 1

    return result.themes.map((extracted, index) => {
        const id = buildCandidateId(result.extension, extracted.label, isMultiple, usedIds)
        usedIds.add(id)

        const themeType = inferThemeType(extracted.uiTheme)
        const rawChain = buildRawChain(extracted)
        const conversion = rawChain ? convertVscodeTheme(rawChain, themeType) : null
        const failureReason = resolveFailureReason(rawChain, conversion)
        const warningCount = conversion ? conversion.safeDefaultNotices.length + conversion.ansiFallbackTokens.length + conversion.repairs.length : 0

        return {
            key: `${index}-${extracted.label}`,
            label: extracted.label,
            id,
            themeType,
            idCollides: existingThemeIds.includes(id),
            warningCount,
            failureReason,
            theme: failureReason || !conversion ? null : buildTheme(id, extracted.label, themeType, conversion, result.extension),
        }
    })
}
