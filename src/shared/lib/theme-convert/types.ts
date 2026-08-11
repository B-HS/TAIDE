export type ThemeTypeArg = 'dark' | 'light'

export type VscodeTokenColorRule = {
    scopes: string[]
    fg?: string
    bold: boolean
    italic: boolean
}

export type VscodeTheme = {
    colors: Record<string, string>
    tokenColors: VscodeTokenColorRule[]
}

export type SyntaxStyle = { fg: string; bold: boolean; italic: boolean }

export type ColorCategory = 'foreground' | 'background' | 'border' | 'status' | 'shadow'

export type ColorMappingEntry = {
    taideKey: string
    category: ColorCategory
    candidates?: string[]
    derive?: (ctx: ResolveContext) => string | undefined
}

export type ResolveContext = {
    vscodeColors: Record<string, string>
    resolved: Record<string, string>
    ansi: AnsiLookup
    type: ThemeTypeArg
}

export const TERMINAL_ANSI_TOKENS = [
    'black',
    'red',
    'green',
    'yellow',
    'blue',
    'magenta',
    'cyan',
    'white',
    'brightBlack',
    'brightRed',
    'brightGreen',
    'brightYellow',
    'brightBlue',
    'brightMagenta',
    'brightCyan',
    'brightWhite',
] as const

export type AnsiLookup = Record<(typeof TERMINAL_ANSI_TOKENS)[number], string>
