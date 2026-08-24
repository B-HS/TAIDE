import { ALPHA_CHANNEL_MAX, HEX_ALPHA_LENGTH, hexToRgb } from '@shared/lib/color'
import { GRAPH_LANE_ANSI_ORDER } from '@shared/lib/theme-convert/ansi-palette'
import type { ColorCategory, ColorMappingEntry, ResolveContext, ThemeTypeArg } from '@shared/lib/theme-convert/types'

const SELF_REF_PREFIX = '@'

export const FAMILY_FALLBACK_SOURCE_KEYS: Record<ColorCategory, string[]> = {
    foreground: ['editor.foreground', 'foreground'],
    background: ['editor.background'],
    border: ['panel.border', 'editorGroup.border', 'contrastBorder'],
    status: [],
    shadow: [],
}

export const SAFE_DEFAULT_COLORS: Record<ThemeTypeArg, Record<ColorCategory, string>> = {
    dark: {
        foreground: '#D4D4D4',
        background: '#1E1E1E',
        border: '#3C3C3C',
        status: '#569CD6',
        shadow: '#00000080',
    },
    light: {
        foreground: '#1E1E1E',
        background: '#FFFFFF',
        border: '#D4D4D4',
        status: '#0066BF',
        shadow: '#00000026',
    },
}

/**
 * VS Code's official `list.hoverBackground` default (src/vs/platform/theme/common/colors/listColors.ts,
 * registerColor('list.hoverBackground', { dark: '#2A2D2E', light: '#F0F0F0', ... })). Themes that either
 * omit `list.hoverBackground` or set it to a value indistinguishable from the row background (same RGB,
 * or alpha 00) fall back to this instead of the generic same-family background fallback — otherwise the
 * hover state silently disappears (docs/theme-system.md §8.2.2).
 */
export const VSCODE_LIST_HOVER_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
    dark: '#2A2D2E',
    light: '#F0F0F0',
}

/**
 * VS Code's official `list.activeSelectionBackground` default (src/vs/platform/theme/common/colors/listColors.ts,
 * registerColor('list.activeSelectionBackground', { dark: '#04395E', light: '#0060C0', ... })). Themes that either
 * omit `list.activeSelectionBackground` or set it to a value indistinguishable from the row background fall back
 * to this instead of the generic same-family background fallback — otherwise the focused selection silently
 * disappears (docs/theme-system.md §8.2.2).
 *
 * The `light` value diverges from VS Code's own default (`#0060C0`). VS Code pairs that dark blue with a
 * dedicated `list.activeSelectionForeground` (white) that TAIDE has no equivalent token for, so rows inherit
 * `app.foreground` (black) and land at ~3.4:1 contrast — below the 4.5:1 AA floor for small text. `#ADD6FF`
 * is VS Code's own light-theme `editor.selectionBackground` default, which clears 13.8:1 against black
 * (docs/theme-system.md §8.2.2).
 */
export const VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
    dark: '#04395E',
    light: '#ADD6FF',
}

/**
 * VS Code's official `list.inactiveSelectionBackground` default (src/vs/platform/theme/common/colors/listColors.ts,
 * registerColor('list.inactiveSelectionBackground', { dark: '#37373D', light: '#E4E6F1', ... })). Same fallback
 * reasoning as `VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT` above, applied to the unfocused selection state.
 */
export const VSCODE_LIST_INACTIVE_SELECTION_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
    dark: '#37373D',
    light: '#E4E6F1',
}

const chain = (taideKey: string, category: ColorCategory, candidates: string[]): ColorMappingEntry => ({ taideKey, category, candidates })
const derived = (taideKey: string, category: ColorCategory, derive: (ctx: ResolveContext) => string | undefined): ColorMappingEntry => ({
    taideKey,
    category,
    derive,
})

const MATCH_HIGHLIGHT_FOREGROUND_CANDIDATES = ['list.highlightForeground', 'editor.findMatchHighlightBackground']

export const COLOR_MAPPING: ColorMappingEntry[] = [
    chain('app.background', 'background', ['editor.background']),
    chain('app.foreground', 'foreground', ['foreground']),
    chain('app.border', 'border', ['panel.border', 'editorGroup.border', 'contrastBorder']),
    chain('app.focusBorder', 'border', ['focusBorder']),
    chain('app.shadow', 'shadow', ['widget.shadow', 'scrollbar.shadow']),
    chain('app.accent', 'status', ['textLink.foreground', 'button.background', 'focusBorder']),

    chain('appSidebar.background', 'background', ['activityBar.background', 'sideBar.background']),
    chain('appSidebar.itemHover', 'background', ['list.hoverBackground']),
    chain('appSidebar.itemActive', 'background', ['list.activeSelectionBackground']),
    chain('appSidebar.iconDefault', 'foreground', ['activityBar.inactiveForeground', 'icon.foreground']),
    derived('appSidebar.iconAgentRunning', 'status', (ctx) => ctx.ansi.green),
    derived('appSidebar.iconAgentWorking', 'status', (ctx) => ctx.ansi.blue),
    derived('appSidebar.iconAgentAwaiting', 'status', (ctx) => ctx.ansi.yellow),
    derived('appSidebar.iconAgentIdle', 'status', (ctx) => ctx.ansi.brightGreen ?? ctx.ansi.green),
    chain('appSidebar.iconAgentUnknown', 'foreground', ['disabledForeground', 'descriptionForeground']),
    chain('appSidebar.badge', 'status', ['activityBarBadge.background', 'badge.background']),

    chain('tabBar.background', 'background', ['editorGroupHeader.tabsBackground']),
    chain('tabBar.tabActiveBackground', 'background', ['tab.activeBackground']),
    chain('tabBar.tabInactiveBackground', 'background', ['tab.inactiveBackground']),
    chain('tabBar.tabActiveForeground', 'foreground', ['tab.activeForeground']),
    chain('tabBar.tabInactiveForeground', 'foreground', ['tab.inactiveForeground']),
    chain('tabBar.tabBorder', 'border', ['tab.border']),
    chain('tabBar.tabActiveIndicator', 'status', ['tab.activeBorderTop', 'tab.activeBorder', 'focusBorder']),
    chain('tabBar.dirtyDot', 'status', ['tab.activeModifiedBorder', 'gitDecoration.modifiedResourceForeground', 'terminal.ansiYellow']),
    chain('tabBar.previewForeground', 'foreground', ['tab.unfocusedActiveForeground', 'tab.inactiveForeground']),
    chain('tabBar.dropTarget', 'status', ['list.dropBackground', 'editorGroup.dropBackground', 'focusBorder']),

    chain('explorer.background', 'background', ['sideBar.background']),
    derived('explorer.itemHover', 'background', (ctx) => {
        const candidate = ctx.vscodeColors['list.hoverBackground']
        const background = ctx.resolved['explorer.background']
        return isUsableListBackground(candidate, background) ? candidate : VSCODE_LIST_HOVER_BACKGROUND_DEFAULT[ctx.type]
    }),
    derived('explorer.itemSelected', 'background', (ctx) => {
        const candidate = ctx.vscodeColors['list.activeSelectionBackground']
        const background = ctx.resolved['explorer.background']
        return isUsableListBackground(candidate, background) ? candidate : VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT[ctx.type]
    }),
    derived('explorer.itemFocused', 'background', (ctx) => {
        const background = ctx.resolved['explorer.background']
        const candidate = [ctx.vscodeColors['list.focusBackground'], ctx.vscodeColors['list.inactiveSelectionBackground']].find((value) =>
            isUsableListBackground(value, background),
        )
        return candidate ?? VSCODE_LIST_INACTIVE_SELECTION_BACKGROUND_DEFAULT[ctx.type]
    }),
    chain('explorer.indentGuide', 'border', ['tree.indentGuidesStroke']),
    chain('explorer.folderIcon', 'foreground', ['icon.foreground', 'sideBar.foreground']),
    chain('explorer.gitModified', 'status', ['gitDecoration.modifiedResourceForeground', 'terminal.ansiYellow']),
    chain('explorer.gitAdded', 'status', ['gitDecoration.addedResourceForeground', 'terminal.ansiGreen']),
    chain('explorer.gitDeleted', 'status', ['gitDecoration.deletedResourceForeground', 'terminal.ansiRed']),
    chain('explorer.gitUntracked', 'status', ['gitDecoration.untrackedResourceForeground', 'terminal.ansiGreen']),
    chain('explorer.gitIgnored', 'status', ['gitDecoration.ignoredResourceForeground', 'terminal.ansiBrightBlack']),

    chain('panel.background', 'background', ['sideBar.background']),
    chain('panel.sectionHeader', 'foreground', ['sideBarSectionHeader.foreground', 'sideBarTitle.foreground']),
    chain('panel.inputBackground', 'background', ['input.background']),
    chain('panel.inputBorder', 'border', ['input.border', 'panelInput.border', 'dropdown.border']),
    /**
     * `panel.matchHighlight` is a foreground token — the palette's and search's matched-character
     * text color, not an overlay background. Its two upstream VS Code candidates are
     * `list.highlightForeground` (an opaque text color when a theme defines it) and
     * `editor.findMatchHighlightBackground` (VS Code's own translucent *overlay* fill, meant to sit
     * as a semi-transparent layer over already-rendered text — never a text color on its own). Many
     * themes omit `list.highlightForeground`, so a plain `chain()` here silently fell through to the
     * overlay value and painted near-invisible text (`github-dark` `#ffd33d22` = 13% opaque,
     * `github-light` `#ffdf5d66` = 40% — `docs/acknowledge/2026-08-20-palette-ux-contract.md` §4.3).
     * {@link isOpaqueForegroundCandidate} rejects any candidate carrying meaningful alpha instead of
     * compositing it (compositing would still hand back a dim-but-"valid" color); once every
     * candidate is rejected this returns `undefined`, which `resolveColorEntry` (`resolve-colors.ts`)
     * routes through the `status` category's fallback chain — empty for `status`
     * ({@link FAMILY_FALLBACK_SOURCE_KEYS}) — straight to {@link SAFE_DEFAULT_COLORS}'s opaque
     * per-theme-type value, the same path already used when every candidate is simply absent.
     */
    derived('panel.matchHighlight', 'status', (ctx) =>
        MATCH_HIGHLIGHT_FOREGROUND_CANDIDATES.map((candidate) => ctx.vscodeColors[candidate]).find(isOpaqueForegroundCandidate),
    ),

    chain('editor.background', 'background', ['editor.background']),
    chain('editor.foreground', 'foreground', ['editor.foreground']),
    chain('editor.lineHighlight', 'background', ['editor.lineHighlightBackground']),
    chain('editor.cursor', 'status', ['editorCursor.foreground', 'terminal.ansiWhite']),
    chain('editor.selection', 'background', ['editor.selectionBackground']),
    chain('editor.inactiveSelection', 'background', ['editor.inactiveSelectionBackground']),
    chain('editor.lineNumber', 'foreground', ['editorLineNumber.foreground']),
    chain('editor.lineNumberActive', 'foreground', ['editorLineNumber.activeForeground']),
    chain('editor.indentGuide', 'border', ['editorIndentGuide.background1', 'editorIndentGuide.background']),
    chain('editor.whitespace', 'foreground', ['editorWhitespace.foreground']),
    chain('editor.bracketMatch', 'border', ['editorBracketMatch.border', 'editorBracketMatch.background']),
    chain('editor.findMatch', 'status', ['editor.findMatchBackground', 'terminal.ansiYellow']),
    chain('editor.findMatchHighlight', 'status', ['editor.findMatchHighlightBackground', `${SELF_REF_PREFIX}editor.findMatch`]),
    chain('editor.hoverBackground', 'background', ['editorHoverWidget.background']),
    chain('editor.widgetBackground', 'background', ['editorWidget.background', 'editorSuggestWidget.background']),
    chain('editor.widgetBorder', 'border', ['editorWidget.border', 'editorHoverWidget.border']),

    chain('editorGutter.addedBackground', 'status', ['editorGutter.addedBackground', 'terminal.ansiGreen']),
    chain('editorGutter.modifiedBackground', 'status', ['editorGutter.modifiedBackground', 'terminal.ansiBlue']),
    chain('editorGutter.deletedBackground', 'status', ['editorGutter.deletedBackground', 'terminal.ansiRed']),

    chain('editorBlame.foreground', 'foreground', ['editorCodeLens.foreground', 'editorInlayHint.foreground', 'descriptionForeground']),
    derived('editorBlame.background', 'background', () => 'transparent'),

    chain('diff.insertedBackground', 'status', ['diffEditor.insertedTextBackground', 'terminal.ansiGreen']),
    chain('diff.insertedLineBackground', 'status', ['diffEditor.insertedLineBackground', `${SELF_REF_PREFIX}diff.insertedBackground`]),
    chain('diff.removedBackground', 'status', ['diffEditor.removedTextBackground', 'terminal.ansiRed']),
    chain('diff.removedLineBackground', 'status', ['diffEditor.removedLineBackground', `${SELF_REF_PREFIX}diff.removedBackground`]),
    chain('diff.border', 'border', ['diffEditor.border', 'editorGroup.border']),

    chain('terminal.background', 'background', ['terminal.background', 'panel.background', 'editor.background']),
    chain('terminal.foreground', 'foreground', ['terminal.foreground', 'foreground']),
    chain('terminal.cursor', 'status', ['terminalCursor.foreground', 'editorCursor.foreground', 'terminal.ansiWhite']),
    chain('terminal.selection', 'background', ['terminal.selectionBackground']),
    chain('terminal.commandBlockBorder', 'border', ['panel.border', `${SELF_REF_PREFIX}app.border`]),
    chain('terminal.linkForeground', 'foreground', ['textLink.foreground', 'editorLink.activeForeground']),

    chain('git.added', 'status', ['gitDecoration.addedResourceForeground', 'terminal.ansiGreen']),
    chain('git.modified', 'status', ['gitDecoration.modifiedResourceForeground', 'terminal.ansiYellow']),
    chain('git.deleted', 'status', ['gitDecoration.deletedResourceForeground', 'terminal.ansiRed']),
    chain('git.renamed', 'status', ['gitDecoration.renamedResourceForeground', 'terminal.ansiBlue']),
    chain('git.untracked', 'status', ['gitDecoration.untrackedResourceForeground', 'terminal.ansiGreen']),
    chain('git.conflicted', 'status', ['gitDecoration.conflictingResourceForeground', 'terminal.ansiMagenta']),
    chain('git.staged', 'status', ['gitDecoration.stageModifiedResourceForeground', `${SELF_REF_PREFIX}git.modified`]),

    ...GRAPH_LANE_ANSI_ORDER.map((ansiName, index) => derived(`graph.lane${index + 1}`, 'status', (ctx) => ctx.ansi[ansiName])),
    derived('graph.refBranch', 'status', (ctx) => ctx.vscodeColors['gitDecoration.modifiedResourceForeground'] ?? ctx.ansi.blue),
    derived('graph.refTag', 'status', (ctx) => ctx.ansi.yellow),
    derived('graph.refHead', 'status', (ctx) => ctx.ansi.green),

    chain('statusIndicator.info', 'status', ['editorInfo.foreground', 'notificationsInfoIcon.foreground', 'terminal.ansiBlue']),
    chain('statusIndicator.warning', 'status', ['editorWarning.foreground', 'notificationsWarningIcon.foreground', 'terminal.ansiYellow']),
    chain('statusIndicator.error', 'status', ['editorError.foreground', 'errorForeground', 'terminal.ansiRed']),
    derived('statusIndicator.success', 'status', (ctx) => ctx.ansi.green ?? ctx.vscodeColors['gitDecoration.addedResourceForeground']),

    chain('menu.background', 'background', ['menu.background', 'dropdown.background']),
    chain('menu.border', 'border', ['menu.border', 'dropdown.border']),
    chain('menu.itemHover', 'background', ['menu.selectionBackground', 'list.hoverBackground']),
    chain('menu.separator', 'border', ['menu.separatorBackground']),

    chain('popover.background', 'background', ['editorWidget.background', 'menu.background']),
    chain('popover.border', 'border', ['editorWidget.border']),
    chain('popover.itemHover', 'background', ['list.hoverBackground']),
    chain('popover.separator', 'border', ['menu.separatorBackground']),

    chain('tooltip.background', 'background', ['editorHoverWidget.background', 'editorWidget.background']),
    chain('tooltip.border', 'border', ['editorHoverWidget.border']),
    chain('tooltip.itemHover', 'background', ['list.hoverBackground']),
    chain('tooltip.separator', 'border', ['menu.separatorBackground']),

    chain('modal.background', 'background', ['editorWidget.background', 'notifications.background']),
    chain('modal.border', 'border', ['editorWidget.border', 'notificationCenter.border']),
    chain('modal.itemHover', 'background', ['list.hoverBackground']),
    chain('modal.separator', 'border', ['menu.separatorBackground']),

    chain('scrollbar.thumb', 'background', ['scrollbarSlider.background']),
    chain('scrollbar.thumbHover', 'background', ['scrollbarSlider.hoverBackground']),
    derived('scrollbar.track', 'background', () => 'transparent'),

    chain('input.background', 'background', ['input.background']),
    chain('input.foreground', 'foreground', ['input.foreground']),
    chain('input.border', 'border', ['input.border', 'dropdown.border']),
    chain('input.placeholder', 'foreground', ['input.placeholderForeground']),
    chain('input.focusBorder', 'border', ['focusBorder']),

    chain('button.background', 'background', ['button.secondaryBackground', 'button.background']),
    chain('button.foreground', 'foreground', ['button.secondaryForeground', 'button.foreground']),
    chain('button.hoverBackground', 'background', ['button.secondaryHoverBackground', 'button.hoverBackground']),
    chain('button.primaryBackground', 'background', ['button.background']),
    chain('button.primaryForeground', 'foreground', ['button.foreground']),

    chain('list.background', 'background', ['sideBar.background']),
    /**
     * `list.hoverBackground`/`list.activeBackground` used to be plain `chain()` entries, so an
     * unusable source value (omitted, alpha 00, or identical to the row background) fell through
     * to the generic same-family fallback (`editor.background` — see {@link FAMILY_FALLBACK_SOURCE_KEYS})
     * instead of {@link VSCODE_LIST_HOVER_BACKGROUND_DEFAULT}/{@link VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT},
     * landing on the same color as `list.background`/`panel.background` and rendering the row
     * completely unselectable-looking — the exact bug `explorer.itemHover`/`explorer.itemSelected`
     * already guard against with {@link isUsableListBackground} a few lines above. `derived()` now
     * applies that same guard here. `list.activeBackground` additionally checks itself against the
     * already-resolved `list.hoverBackground` (not just the row background): several themes
     * (ayu, gruvbox, night-owl-light, one-dark-pro, vitesse) define `list.activeSelectionBackground`
     * identical to their own `list.hoverBackground`, which is a legitimate one-color hover/selection
     * design upstream but leaves TAIDE's focused-selection state visually indistinguishable from a
     * transient mouse hover — VS Code convention expects the active selection to read as the
     * stronger of the two. See `docs/acknowledge/2026-08-20-theme-list-colors-contract.md`.
     */
    derived('list.hoverBackground', 'background', (ctx) => {
        const candidate = ctx.vscodeColors['list.hoverBackground']
        const background = ctx.resolved['list.background']
        return isUsableListBackground(candidate, background) ? candidate : VSCODE_LIST_HOVER_BACKGROUND_DEFAULT[ctx.type]
    }),
    derived('list.activeBackground', 'background', (ctx) => {
        const candidate = ctx.vscodeColors['list.activeSelectionBackground']
        const background = ctx.resolved['list.background']
        const hover = ctx.resolved['list.hoverBackground']
        const usable = isUsableListBackground(candidate, background) && isUsableListBackground(candidate, hover)
        return usable ? candidate : VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT[ctx.type]
    }),
    chain('list.foreground', 'foreground', ['sideBar.foreground', 'foreground']),
]

/**
 * A `list.*Background` candidate (hover/activeSelection/inactiveSelection) is unusable when it is fully
 * transparent (alpha 00) or shares the exact same RGB as the row background — both render as "no state at
 * all" (docs/theme-system.md §8.2.2). Themes that hit either case fall back to VS Code's official default
 * for that token instead.
 */
export const isUsableListBackground = (candidateHex: string | undefined, backgroundHex: string | undefined): candidateHex is string => {
    if (!candidateHex) return false
    const alpha = candidateHex.length === HEX_ALPHA_LENGTH ? Number.parseInt(candidateHex.slice(7, HEX_ALPHA_LENGTH), 16) : ALPHA_CHANNEL_MAX
    if (alpha === 0) return false
    if (!backgroundHex) return true
    const candidateRgb = hexToRgb(candidateHex)
    const backgroundRgb = hexToRgb(backgroundHex)
    if (!candidateRgb || !backgroundRgb) return true
    return candidateRgb.r !== backgroundRgb.r || candidateRgb.g !== backgroundRgb.g || candidateRgb.b !== backgroundRgb.b
}

const SHORT_HEX_ALPHA_LENGTH = 5
const SHORT_ALPHA_CHANNEL_MAX = 15

/**
 * A foreground color candidate (as opposed to {@link isUsableListBackground}'s row-background
 * concern) is usable only when it is fully opaque — a translucent value is meant to sit as an
 * overlay atop already-rendered content, not to paint text directly. Carries "meaningful" alpha
 * when it is the 8-digit hex form (`#rrggbbaa`) and its alpha channel reads below `ff`, or the
 * 4-digit shorthand form (`#rgba`) and its alpha nibble reads below `f`; anything else (undefined,
 * a 3/6-digit hex, or an alpha channel/nibble that rounds up to fully opaque) is opaque and usable
 * as-is. Callers in this module always pass values already normalized by
 * `expandVscodeHex` (`jsonc.ts`, applied during `merge.ts`), which expands 3/4-digit shorthand to
 * 6/8-digit before this runs — the 4-digit branch here exists so the predicate itself stays correct
 * for any caller, normalized or not, rather than silently trusting that upstream step. Used by
 * `panel.matchHighlight` above to reject VS Code's translucent `editor.findMatchHighlightBackground`
 * overlay color instead of painting it (or a composited dim version of it) directly as text.
 */
export const isOpaqueForegroundCandidate = (candidateHex: string | undefined): candidateHex is string => {
    if (!candidateHex) return false
    if (candidateHex.length === SHORT_HEX_ALPHA_LENGTH) return Number.parseInt(candidateHex.slice(-1), 16) === SHORT_ALPHA_CHANNEL_MAX
    if (candidateHex.length !== HEX_ALPHA_LENGTH) return true
    return Number.parseInt(candidateHex.slice(7, HEX_ALPHA_LENGTH), 16) === ALPHA_CHANNEL_MAX
}
