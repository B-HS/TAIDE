import { getCurrentWindow } from '@tauri-apps/api/window'
import type { ThemeType } from '@shared/api/bindings'

export const applyWindowAppearance = (type: ThemeType) => {
    void getCurrentWindow().setTheme(type)
}
