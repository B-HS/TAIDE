import type { ITheme } from '@xterm/xterm'
import type { ResolvedTheme } from '@shared/api/bindings'

const ANSI_KEYS = [
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

export const toXtermTheme = (theme: ResolvedTheme) => {
    const terminal = theme.terminal
    const result: ITheme = {
        background: terminal.background,
        foreground: terminal.foreground,
        cursor: terminal.cursor,
        selectionBackground: terminal.selection,
    }

    for (const key of ANSI_KEYS) {
        const value = terminal[key]
        if (value) result[key] = value
    }

    return result
}
