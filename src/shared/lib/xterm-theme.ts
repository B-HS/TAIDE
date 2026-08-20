import type { ITheme } from '@xterm/xterm'
import type { ResolvedTheme } from '@shared/api/bindings'
import { TERMINAL_ANSI_TOKENS } from '@shared/lib/theme-convert/types'

export const toXtermTheme = (theme: ResolvedTheme) => {
    const terminal = theme.terminal
    const result: ITheme = {
        background: terminal.background,
        foreground: terminal.foreground,
        cursor: terminal.cursor,
        selectionBackground: terminal.selection,
    }

    for (const key of TERMINAL_ANSI_TOKENS) {
        const value = terminal[key]
        if (value) result[key] = value
    }

    return result
}
