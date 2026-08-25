import { compositeOverBackground, hexToRgb } from '@shared/lib/color'
import { isDistinctFromBodyForeground, isOpaqueForegroundCandidate } from '@shared/lib/theme-convert/mapping-tables'

const RGB_CHANNEL_MAX = 255
const SRGB_LINEAR_THRESHOLD = 0.03928
const SRGB_LINEAR_DIVISOR = 12.92
const SRGB_GAMMA_OFFSET = 0.055
const SRGB_GAMMA_DIVISOR = 1.055
const SRGB_GAMMA_EXPONENT = 2.4
const LUMINANCE_WEIGHT_R = 0.2126
const LUMINANCE_WEIGHT_G = 0.7152
const LUMINANCE_WEIGHT_B = 0.0722
const CONTRAST_RATIO_OFFSET = 0.05
const MIN_CONTRAST_RATIO = 3

const srgbChannelToLinear = (channel: number) => {
    const normalized = channel / RGB_CHANNEL_MAX
    return normalized <= SRGB_LINEAR_THRESHOLD
        ? normalized / SRGB_LINEAR_DIVISOR
        : ((normalized + SRGB_GAMMA_OFFSET) / SRGB_GAMMA_DIVISOR) ** SRGB_GAMMA_EXPONENT
}

const relativeLuminance = (hex: string): number | null => {
    const rgb = hexToRgb(hex)
    if (!rgb) return null
    return (
        LUMINANCE_WEIGHT_R * srgbChannelToLinear(rgb.r) +
        LUMINANCE_WEIGHT_G * srgbChannelToLinear(rgb.g) +
        LUMINANCE_WEIGHT_B * srgbChannelToLinear(rgb.b)
    )
}

export const contrastRatio = (hexA: string, hexB: string): number | null => {
    const luminanceA = relativeLuminance(hexA)
    const luminanceB = relativeLuminance(hexB)
    if (luminanceA === null || luminanceB === null) return null
    const lighter = Math.max(luminanceA, luminanceB)
    const darker = Math.min(luminanceA, luminanceB)
    return (lighter + CONTRAST_RATIO_OFFSET) / (darker + CONTRAST_RATIO_OFFSET)
}

const foregroundContrastRatio = (foregroundHex: string, backgroundHex: string) =>
    contrastRatio(compositeOverBackground(foregroundHex, backgroundHex), backgroundHex)

const CONTRAST_PAIRS: readonly { label: string; foregroundKey: string; backgroundKey: string }[] = [
    { label: 'app', foregroundKey: 'app.foreground', backgroundKey: 'app.background' },
    { label: 'editor', foregroundKey: 'editor.foreground', backgroundKey: 'editor.background' },
    { label: 'panel', foregroundKey: 'panel.sectionHeader', backgroundKey: 'panel.background' },
    { label: 'tooltip', foregroundKey: 'app.foreground', backgroundKey: 'tooltip.background' },
    { label: 'matchHighlight', foregroundKey: 'panel.matchHighlight', backgroundKey: 'panel.background' },
]

const CONTRAST_REPAIR_BACKGROUND_CANDIDATES: Record<string, string[]> = {
    'tooltip.background': ['editorWidget.background', 'menu.background', 'dropdown.background'],
}

const CONTRAST_REPAIR_FOREGROUND_CANDIDATES: Record<string, string[]> = {
    'app.foreground': ['editor.foreground'],
    'panel.sectionHeader': ['editor.foreground'],
    'panel.matchHighlight': [
        'textLink.foreground',
        'button.background',
        'focusBorder',
        'activityBarBadge.background',
        'badge.background',
        'tab.activeBorderTop',
        'editor.foreground',
        'foreground',
    ],
}

const MATCH_HIGHLIGHT_FOREGROUND_KEY = 'panel.matchHighlight'
const MATCH_HIGHLIGHT_REPAIR_FALLBACK_NOTICE = ', 본문 전경과 동일색 — 구별 가능한 후보 없음'

const meetsMinContrast = (foregroundHex: string, backgroundHex: string) =>
    (foregroundContrastRatio(foregroundHex, backgroundHex) ?? 0) >= MIN_CONTRAST_RATIO

export const repairContrastPairs = (colors: Record<string, string>, vscodeColors: Record<string, string>) => {
    const repairs: string[] = []
    let repairedColors = colors

    for (const pair of CONTRAST_PAIRS) {
        const foreground = repairedColors[pair.foregroundKey]
        const background = repairedColors[pair.backgroundKey]
        const ratio = foregroundContrastRatio(foreground, background)
        if (ratio !== null && ratio >= MIN_CONTRAST_RATIO) continue

        const backgroundCandidates = CONTRAST_REPAIR_BACKGROUND_CANDIDATES[pair.backgroundKey] ?? []
        const repairedBackground = backgroundCandidates.map((key) => vscodeColors[key]).find((value) => value && meetsMinContrast(foreground, value))
        if (repairedBackground) {
            repairedColors = { ...repairedColors, [pair.backgroundKey]: repairedBackground }
            repairs.push(`${pair.backgroundKey}: ${background} -> ${repairedBackground} (${pair.label} 대비 확보)`)
            continue
        }

        const foregroundCandidateValues = (CONTRAST_REPAIR_FOREGROUND_CANDIDATES[pair.foregroundKey] ?? []).map((key) => vscodeColors[key])
        const isMatchHighlight = pair.foregroundKey === MATCH_HIGHLIGHT_FOREGROUND_KEY
        const distinctForeground = isMatchHighlight
            ? foregroundCandidateValues.find(
                  (value) =>
                      isOpaqueForegroundCandidate(value) &&
                      meetsMinContrast(value, background) &&
                      isDistinctFromBodyForeground(value, repairedColors['app.foreground']),
              )
            : undefined
        const repairedForeground = distinctForeground ?? foregroundCandidateValues.find((value) => value && meetsMinContrast(value, background))
        if (!repairedForeground) continue

        repairedColors = { ...repairedColors, [pair.foregroundKey]: repairedForeground }
        const usedDistinctnessFallback = isMatchHighlight && !distinctForeground
        repairs.push(
            `${pair.foregroundKey}: ${foreground} -> ${repairedForeground} (${pair.label} 대비 확보${usedDistinctnessFallback ? MATCH_HIGHLIGHT_REPAIR_FALLBACK_NOTICE : ''})`,
        )
    }

    return { colors: repairedColors, repairs }
}

export const validateOutputColors = (colors: Record<string, string>) => {
    const errors: string[] = []

    if (colors['app.foreground'] === colors['app.background']) {
        errors.push(`app.foreground와 app.background가 동일한 색(${colors['app.foreground']})입니다`)
    }

    for (const pair of CONTRAST_PAIRS) {
        const foreground = colors[pair.foregroundKey]
        const background = colors[pair.backgroundKey]
        const ratio = foregroundContrastRatio(foreground, background)
        if (ratio === null || ratio < MIN_CONTRAST_RATIO) {
            errors.push(
                `${pair.label} 대비 부족: ${pair.foregroundKey}(${foreground}) vs ${pair.backgroundKey}(${background}) = ${ratio?.toFixed(2) ?? 'N/A'} (최소 ${MIN_CONTRAST_RATIO})`,
            )
        }
    }

    return errors
}
