import type { AnsiLookup, ThemeTypeArg } from '@shared/lib/theme-convert/types'

export const TERMINAL_MIRRORED_TOKENS = {
    background: 'terminal.background',
    foreground: 'terminal.foreground',
    cursor: 'terminal.cursor',
    selection: 'terminal.selection',
} as const

export const VSCODE_DEFAULT_ANSI_PALETTE: Record<ThemeTypeArg, AnsiLookup> = {
    dark: {
        black: '#000000',
        red: '#cd3131',
        green: '#0dbc79',
        yellow: '#e5e510',
        blue: '#2472c8',
        magenta: '#bc3fbc',
        cyan: '#11a8cd',
        white: '#e5e5e5',
        brightBlack: '#666666',
        brightRed: '#f14c4c',
        brightGreen: '#23d18b',
        brightYellow: '#f5f543',
        brightBlue: '#3b8eea',
        brightMagenta: '#d670d6',
        brightCyan: '#29b8db',
        brightWhite: '#e5e5e5',
    },
    light: {
        black: '#000000',
        red: '#cd3131',
        green: '#107c10',
        yellow: '#949800',
        blue: '#0451a5',
        magenta: '#bc05bc',
        cyan: '#0598bc',
        white: '#555555',
        brightBlack: '#666666',
        brightRed: '#cd3131',
        brightGreen: '#14ce14',
        brightYellow: '#b5ba00',
        brightBlue: '#0451a5',
        brightMagenta: '#bc05bc',
        brightCyan: '#0598bc',
        brightWhite: '#a5a5a5',
    },
}

export const GRAPH_LANE_ANSI_ORDER = [
    'blue',
    'green',
    'yellow',
    'magenta',
    'cyan',
    'red',
    'brightBlue',
    'brightGreen',
    'brightYellow',
    'brightMagenta',
    'brightCyan',
    'brightRed',
] as const
