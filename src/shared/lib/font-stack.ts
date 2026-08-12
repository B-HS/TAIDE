const MONOSPACE_FONT_FALLBACK_STACK = ['ui-monospace', 'SFMono-Regular', 'Menlo', '"Apple SD Gothic Neo"', 'monospace']

const quoteFontFamily = (fontFamily: string) => (fontFamily.includes(' ') ? `"${fontFamily}"` : fontFamily)

export const buildMonospaceFontStack = (fontFamily: string | null) => {
    if (!fontFamily) return MONOSPACE_FONT_FALLBACK_STACK.join(', ')
    return [quoteFontFamily(fontFamily), ...MONOSPACE_FONT_FALLBACK_STACK].join(', ')
}
