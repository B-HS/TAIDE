import { expandVscodeHex } from '@shared/lib/theme-convert/jsonc'
import type { VscodeTheme, VscodeTokenColorRule } from '@shared/lib/theme-convert/types'

export const readVscodeTheme = (raw: Record<string, unknown>): VscodeTheme => {
    const rawColors = typeof raw.colors === 'object' && raw.colors !== null ? (raw.colors as Record<string, unknown>) : {}
    const colors = Object.fromEntries(
        Object.entries(rawColors)
            .filter((entry): entry is [string, string] => typeof entry[1] === 'string')
            .map(([key, value]) => [key, expandVscodeHex(value)]),
    )
    const rawTokenColors = Array.isArray(raw.tokenColors) ? raw.tokenColors : []

    const tokenColors: VscodeTokenColorRule[] = rawTokenColors.flatMap((entry) => {
        if (typeof entry !== 'object' || entry === null) return []
        const { scope, settings } = entry as {
            scope?: string | string[]
            settings?: { foreground?: string; background?: string; fontStyle?: string }
        }
        if (!settings) return []
        const scopeList = scope === undefined ? [] : Array.isArray(scope) ? scope : scope.split(',')
        const scopes = scopeList.map((value) => value.trim()).filter((value) => value.length > 0)
        const fontStyleText = settings.fontStyle ?? ''
        const fg = settings.foreground === undefined ? undefined : expandVscodeHex(settings.foreground)
        const background = settings.background === undefined ? undefined : expandVscodeHex(settings.background)
        return [
            {
                scopes,
                fg,
                background,
                fontStyle: settings.fontStyle,
                bold: fontStyleText.includes('bold'),
                italic: fontStyleText.includes('italic'),
            },
        ]
    })

    return { colors, tokenColors }
}

export const mergeVscodeThemeChain = (rawChain: Record<string, unknown>[]): VscodeTheme =>
    rawChain
        .map(readVscodeTheme)
        .reduce((merged, theme) => ({ colors: { ...merged.colors, ...theme.colors }, tokenColors: [...merged.tokenColors, ...theme.tokenColors] }))
