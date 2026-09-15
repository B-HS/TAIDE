/**
 * Minimum CIE76 ΔE*ab (see `deltaE76` in `@shared/lib/color`) a state color must clear against the
 * container it is painted on before the state reads as a state at all. `2.3` is the commonly cited
 * CIE76 "just noticeable difference" threshold and is already the codebase's distinctness floor
 * (`mapping-tables.ts`'s `MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E`), so the two lints agree on what
 * "a different color" means. The bundled themes' measured distribution (recorded per axis in
 * `docs/theme-system.md` §8.5) makes this cut land inside an empty gap: across the 47 themes and
 * every axis this threshold guards, the highest sub-threshold measurement is 2.26
 * (`tokyo-night-light`'s `listHoverOnActiveTab`) and the lowest measurement that clears it is 2.31
 * (`palenight`'s `switchCheckedVsUncheckedTrack`), so no theme sits on the line.
 */
export const STATE_MIN_DISTINCT_DELTA_E = 2.3

/**
 * Relaxed counterpart of {@link STATE_MIN_DISTINCT_DELTA_E} for overlays a code editor is expected
 * to keep faint — the current-line band, the unfocused (inactive) selection, and the secondary
 * find-match tint all sit under the caret while the user reads text, so upstream themes deliberately
 * tune them to be barely-there. `1` is the classical "not perceptible by the human eye" bound for
 * CIE76, i.e. it only rejects a state that has collapsed into its container outright while passing
 * every deliberately-subtle value: on the bundled catalog the lowest non-zero measurements on these
 * axes are 2.17 (`editor.lineHighlight`), 2.28 (`editor.inactiveSelection`) and 7.83
 * (`editor.findMatchHighlight`), all comfortably above it.
 */
export const SUBTLE_STATE_MIN_DISTINCT_DELTA_E = 1

/**
 * Minimum alpha `app.shadow` must carry for the separation it exists to draw to survive. The token
 * is the color of every floating surface's drop shadow (`global.css`'s `shadow-overlay` utilities)
 * and, at half strength, of the modal scrim (`modal-scrim`), so a fully transparent value means a
 * dialog is pasted flat onto the content behind it with nothing marking it as modal.
 *
 * `0.149` is VS Code's own light-theme `widget.shadow` default (`#00000026`, 38/255) and is already
 * this converter's light `SAFE_DEFAULT_COLORS.shadow` (`mapping-tables.ts`), i.e. the faintest
 * shadow the platform itself ships rather than a value fitted to this catalog. The 47 bundled themes
 * measure at exactly 0 (4 themes, each of which declares a shadow color and then zeroes its alpha),
 * 0.071, 0.125 (x2), and then 0.149 upward for the remaining 40 — so the bound lifts the four no-op
 * shadows plus the three that sit below the platform floor and leaves every theme that chose a real
 * shadow untouched, `#00000026` included. Per-theme values are in `docs/theme-system.md` §8.5.
 */
export const APP_SHADOW_MIN_ALPHA = 0.149

export type StateDistinctnessPair = {
    label: string
    stateKey: string
    containerKey: string
    minDeltaE: number
    /**
     * The opaque surface `containerKey` is itself painted on, used to resolve a translucent
     * container before the state is composited over it. Without it a container like
     * `explorer.itemHover` (20 of the 47 bundled themes ship it as an `#rrggbbaa` overlay of the very
     * color `explorer.itemSelected` uses opaquely — dracula's `#44475A75` vs `#44475A`) would be read
     * at its raw RGB and score ΔE 0 against a state that is plainly distinguishable on screen.
     * Omitted when the container is always opaque in practice (`editor.background`,
     * `terminal.background`, `list.background`, `explorer.background`, `panel.background`,
     * `app.background`, `menu.background` carry no alpha in any bundled theme).
     */
    surfaceKey?: string
    /**
     * A second state token that may carry the distinction instead of `stateKey`. A pair passes when
     * either token clears `minDeltaE` against the container, which is what lets a deliberately flat
     * tab strip stay flat as long as it marks the active tab some other way (see the `tabActive`
     * pairs below and their `tabBar.tabActiveIndicator` alternative).
     */
    alternativeStateKey?: string
    /**
     * Marks a pair whose `containerKey` is not the surface `stateKey` is painted on but a *sibling*
     * state of the same element — the selected row next to the hovered one, the checked switch track
     * next to the unchecked one. Both are drawn on `surfaceKey` (required for these pairs), so both
     * are composited over it independently and then compared, instead of compositing the state over
     * the container as a normal pair does.
     *
     * Without this the two are measured as if one were painted on top of the other, which silently
     * passes the exact collapse the pair exists to catch: ayu's `explorer.itemSelected` and
     * `explorer.itemHover` are the same `#rrggbbaa` overlay, so laying the state over the container
     * tints it a second time and scores ΔE 6+ for two rows that render identically on screen.
     */
    sibling?: boolean
}

/**
 * Every (state color, container it is drawn on) pair TAIDE's UI actually renders, derived from the
 * code rather than from the token vocabulary: `global.css`'s `@theme inline` block maps each
 * `--taide-*` token to the Tailwind color the components name, and `monaco/theme.ts`'s
 * `MONACO_COLOR_SOURCE` / `xterm-theme.ts` map the rest onto the two embedded renderers. A pair earns
 * a row only when both tokens can be on screen at the same time (or one is painted directly on the
 * other) and collapsing them destroys information the user has to read — so alternate renderings of
 * one row that are never co-visible (`explorer.itemSelected` vs `explorer.itemFocused`, which differ
 * only by whether the tree has focus) and cosmetic affordances on non-informational chrome
 * (`scrollbar.thumbHover` vs `scrollbar.thumb`, a single thumb that cannot be compared with itself)
 * are deliberately absent; `docs/theme-system.md` §8.5 records those exclusions and their reasons.
 *
 * This table is the single source of truth for the Rust catalog lint in
 * `src-tauri/src/domain/theme/service.rs`, which replicates it verbatim (plain literals, one object
 * per entry, no computed keys) and is kept from drifting by a label/count comparison test against
 * this file — the same arrangement `service.rs`'s existing token-list comparison against
 * `src/entities/theme/theme-tokens.ts` uses.
 */
export const STATE_DISTINCTNESS_PAIRS: readonly StateDistinctnessPair[] = [
    /** Monaco `editor.selectionBackground` — the drag-selected text band inside the editor viewport. */
    { label: 'editorSelection', stateKey: 'editor.selection', containerKey: 'editor.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** Monaco `editor.inactiveSelectionBackground` — the same band once focus moves to another pane. */
    {
        label: 'editorInactiveSelection',
        stateKey: 'editor.inactiveSelection',
        containerKey: 'editor.background',
        minDeltaE: SUBTLE_STATE_MIN_DISTINCT_DELTA_E,
    },
    /** Monaco `editor.lineHighlightBackground` — the band marking the line the caret is on. */
    { label: 'editorCurrentLine', stateKey: 'editor.lineHighlight', containerKey: 'editor.background', minDeltaE: SUBTLE_STATE_MIN_DISTINCT_DELTA_E },
    /** Monaco `editor.findMatchBackground` — the currently focused find/replace hit. */
    { label: 'editorFindMatch', stateKey: 'editor.findMatch', containerKey: 'editor.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** Monaco `editor.findMatchHighlightBackground` — the remaining find/replace hits. */
    {
        label: 'editorFindMatchHighlight',
        stateKey: 'editor.findMatchHighlight',
        containerKey: 'editor.background',
        minDeltaE: SUBTLE_STATE_MIN_DISTINCT_DELTA_E,
    },
    /** Monaco `editorBracketMatch.border` — the box drawn around the bracket pairing the caret. */
    { label: 'editorBracketMatch', stateKey: 'editor.bracketMatch', containerKey: 'editor.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** Monaco `editorCursor.foreground` — the caret itself. */
    { label: 'editorCursor', stateKey: 'editor.cursor', containerKey: 'editor.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },

    /** xterm `selectionBackground` (`xterm-theme.ts`) — the selected region of terminal output. */
    { label: 'terminalSelection', stateKey: 'terminal.selection', containerKey: 'terminal.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** xterm `cursor` (`xterm-theme.ts`) — the terminal caret block. */
    { label: 'terminalCursor', stateKey: 'terminal.cursor', containerKey: 'terminal.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },

    /** Monaco `list.hoverBackground` on `dropdown.listBackground` (`monaco/theme.ts:99,155`) — the hovered row of a suggest/quick-input/tree list. */
    { label: 'listHover', stateKey: 'list.hoverBackground', containerKey: 'list.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `data-[selected=true]:bg-list-active-background` on `--color-list-background` (`shared/ui/command.tsx:112`) — the selected palette row. */
    { label: 'listActive', stateKey: 'list.activeBackground', containerKey: 'list.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** Selected vs merely hovered row of the same list — both are on screen whenever the pointer moves off the selection. */
    {
        label: 'listActiveVsHover',
        stateKey: 'list.activeBackground',
        containerKey: 'list.hoverBackground',
        surfaceKey: 'list.background',
        sibling: true,
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** `hover:bg-list-hover-background` on the active tab (`shared/constants/ui-class.ts` `ICON_BUTTON_CLASS`, `features/tab/tab-item.tsx:85`) — the tab's close/pin button. */
    {
        label: 'listHoverOnActiveTab',
        stateKey: 'list.hoverBackground',
        containerKey: 'tabBar.tabActiveBackground',
        surfaceKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** The same close button on an inactive tab (`features/tab/tab-item.tsx:55,85`). */
    {
        label: 'listHoverOnInactiveTab',
        stateKey: 'list.hoverBackground',
        containerKey: 'tabBar.tabInactiveBackground',
        surfaceKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },

    /** `hover:bg-explorer-item-hover` on `bg-explorer-background` (`features/explorer/file-tree.tsx:284`, `features/git/status-row-item.tsx:55`). */
    { label: 'explorerItemHover', stateKey: 'explorer.itemHover', containerKey: 'explorer.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `bg-explorer-item-selected` on `bg-explorer-background` (`features/explorer/file-tree-row.tsx:63`) — selected row in a focused tree. */
    { label: 'explorerItemSelected', stateKey: 'explorer.itemSelected', containerKey: 'explorer.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `bg-explorer-item-focused` on `bg-explorer-background` (`features/explorer/file-tree-row.tsx:63`) — selected row once the tree loses focus. */
    { label: 'explorerItemFocused', stateKey: 'explorer.itemFocused', containerKey: 'explorer.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** Selected vs hovered file-tree row — pointing at a different file than the selected one shows both at once. */
    {
        label: 'explorerSelectedVsHover',
        stateKey: 'explorer.itemSelected',
        containerKey: 'explorer.itemHover',
        surfaceKey: 'explorer.background',
        sibling: true,
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** The same tree-row states on a `bg-panel-background` panel: problems, search results and outline (`features/problems/problem-row.tsx:34`, `features/search/search-match-row.tsx:33`, `features/outline/outline-symbol-row.tsx:65`). */
    { label: 'explorerHoverOnPanel', stateKey: 'explorer.itemHover', containerKey: 'panel.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `bg-explorer-item-selected` on the same panels (`features/problems/problem-severity-filter.tsx:37`, `features/search/search-option-toggles.tsx:21`, `features/outline/outline-symbol-row.tsx:66`). */
    { label: 'explorerSelectedOnPanel', stateKey: 'explorer.itemSelected', containerKey: 'panel.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `bg-explorer-item-focused` on the same panels — the outline row that is selected while the tree is unfocused (`features/outline/outline-symbol-row.tsx:66`). */
    { label: 'explorerFocusedOnPanel', stateKey: 'explorer.itemFocused', containerKey: 'panel.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `hover:bg-explorer-item-hover` on the status bar strip (`features/window/status-bar.tsx:97,113,165`), which is `bg-app-sidebar-background`. */
    {
        label: 'explorerHoverOnStatusBar',
        stateKey: 'explorer.itemHover',
        containerKey: 'appSidebar.background',
        surfaceKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** `bg-explorer-item-selected` on that same strip — the pressed problems-panel toggle (`features/window/status-bar.tsx:113`). */
    {
        label: 'explorerSelectedOnStatusBar',
        stateKey: 'explorer.itemSelected',
        containerKey: 'appSidebar.background',
        surfaceKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** `hover:bg-explorer-item-hover` on an editor widget surface — the spreadsheet preview's sheet tabs (`features/preview/spreadsheet-preview.tsx:54,67`). */
    {
        label: 'explorerHoverOnEditorWidget',
        stateKey: 'explorer.itemHover',
        containerKey: 'editor.widgetBackground',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },

    /** `hover:bg-app-sidebar-item-hover` on `bg-app-sidebar-background` (`widgets/app-sidebar/app-sidebar.tsx:81,122`). */
    {
        label: 'sidebarItemHover',
        stateKey: 'appSidebar.itemHover',
        containerKey: 'appSidebar.background',
        surfaceKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** `bg-app-sidebar-item-active` on `bg-app-sidebar-background` — the active activity-bar / settings-nav entry (`features/settings/settings-toc.tsx:27`). */
    {
        label: 'sidebarItemActive',
        stateKey: 'appSidebar.itemActive',
        containerKey: 'appSidebar.background',
        surfaceKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** Active vs hovered entry of the same nav list — hovering a second entry puts both states side by side. */
    {
        label: 'sidebarActiveVsHover',
        stateKey: 'appSidebar.itemActive',
        containerKey: 'appSidebar.itemHover',
        surfaceKey: 'appSidebar.background',
        sibling: true,
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** `bg-app-sidebar-item-active` on a settings `Card` (`--card` = `panel.background`): the picked theme / shell profile (`features/settings/theme-picker.tsx:43`). */
    { label: 'sidebarActiveOnCard', stateKey: 'appSidebar.itemActive', containerKey: 'panel.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `hover:bg-app-sidebar-item-hover` on that same settings `Card` (`features/settings/shell-profile-list.tsx:23`). */
    { label: 'sidebarHoverOnCard', stateKey: 'appSidebar.itemHover', containerKey: 'panel.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `hover:bg-app-sidebar-item-hover` on an editor surface — the breadcrumbs bar and markdown preview code spans (`widgets/editor-pane/breadcrumbs-bar.tsx:178`, `features/editor/breadcrumb-segment.tsx:27`, `features/editor/markdown-preview.tsx:6,9`). */
    { label: 'sidebarHoverOnEditor', stateKey: 'appSidebar.itemHover', containerKey: 'editor.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `hover:bg-app-sidebar-item-hover` on `bg-app-background` — the settings table of contents and the welcome screen's recent list (`widgets/settings-view/settings-view.tsx:158`, `features/settings/settings-toc.tsx:26`, `features/welcome/welcome-screen.tsx:88`). */
    { label: 'sidebarHoverOnApp', stateKey: 'appSidebar.itemHover', containerKey: 'app.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `bg-app-sidebar-item-active` on `bg-app-background` — the selected settings section and the theme editor's bold/italic toggles (`features/settings/settings-toc.tsx:27`, `widgets/theme-editor/theme-editor.tsx:170`, `features/theme/syntax-token-row.tsx:36,47`). */
    { label: 'sidebarActiveOnApp', stateKey: 'appSidebar.itemActive', containerKey: 'app.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `hover:bg-app-sidebar-item-hover` on the tab strip — the new-tab menu button (`widgets/editor-area/pane-tab-bar.tsx:293`, `features/tab/tab-bar-add-menu.tsx:9`). */
    {
        label: 'sidebarHoverOnTabBar',
        stateKey: 'appSidebar.itemHover',
        containerKey: 'tabBar.background',
        surfaceKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** `text-app-sidebar-badge` / Monaco `badge.background` drawn on the activity bar and status bar (`features/settings/remote-password-row.tsx:36`). */
    {
        label: 'sidebarBadge',
        stateKey: 'appSidebar.badge',
        containerKey: 'appSidebar.background',
        surfaceKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },

    /** `bg-tab-bar-tab-active-background` against the strip behind the tabs (`features/tab/tab-item.tsx:54`, `widgets/editor-area/pane-tab-bar.tsx:275`). */
    {
        label: 'tabActive',
        stateKey: 'tabBar.tabActiveBackground',
        containerKey: 'tabBar.background',
        surfaceKey: 'app.background',
        alternativeStateKey: 'tabBar.tabActiveIndicator',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** Active vs inactive tab (`features/tab/tab-item.tsx:54-56`) — the `bg-tab-bar-tab-active-indicator` strip is the accepted alternative cue. */
    {
        label: 'tabActiveVsInactive',
        stateKey: 'tabBar.tabActiveBackground',
        containerKey: 'tabBar.tabInactiveBackground',
        surfaceKey: 'app.background',
        alternativeStateKey: 'tabBar.tabActiveIndicator',
        sibling: true,
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },

    /** `data-[state=checked]:bg-primary` (= `button.primaryBackground`) on a settings `Card` (`shared/ui/switch.tsx:11`, `shared/ui/checkbox.tsx:12`). */
    {
        label: 'switchCheckedTrackOnCard',
        stateKey: 'button.primaryBackground',
        containerKey: 'panel.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** The switch thumb is `bg-background` (= `app.background`, `shared/ui/switch.tsx:17`), so the checked track has to stay distinct from it. */
    {
        label: 'switchCheckedTrackVsThumb',
        stateKey: 'button.primaryBackground',
        containerKey: 'app.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** `data-[state=unchecked]:bg-input` (= `input.border`) on a settings `Card` (`shared/ui/switch.tsx:11`). */
    { label: 'switchUncheckedTrackOnCard', stateKey: 'input.border', containerKey: 'panel.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** The same thumb against the unchecked track. */
    { label: 'switchUncheckedTrackVsThumb', stateKey: 'input.border', containerKey: 'app.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** Checked vs unchecked track — the entire on/off read of a settings toggle, and of a row of toggles seen together. */
    {
        label: 'switchCheckedVsUncheckedTrack',
        stateKey: 'button.primaryBackground',
        containerKey: 'input.border',
        surfaceKey: 'panel.background',
        sibling: true,
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },

    /** Monaco `button.hoverBackground` / `button.secondaryHoverBackground` against `button.background` (`monaco/theme.ts:167,171`). */
    {
        label: 'buttonHover',
        stateKey: 'button.hoverBackground',
        containerKey: 'button.background',
        surfaceKey: 'panel.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },

    /** Monaco `input.border` against `input.background` (`monaco/theme.ts:60-62`) — the find/replace field outline. */
    {
        label: 'inputBorder',
        stateKey: 'input.border',
        containerKey: 'input.background',
        surfaceKey: 'panel.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },
    /** `border-panel-input-border` on `bg-panel-input-background` — every settings text/number field (`features/settings/text-field.tsx:30`). */
    {
        label: 'panelInputBorder',
        stateKey: 'panel.inputBorder',
        containerKey: 'panel.inputBackground',
        surfaceKey: 'panel.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },

    /** `--ring` (= `app.focusBorder`) on the app background — `focus-visible:ring-ring/50` on buttons, switches and checkboxes. */
    { label: 'focusBorderOnApp', stateKey: 'app.focusBorder', containerKey: 'app.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `border-app-focus-border` on a settings `Card` (`features/settings/theme-picker.tsx:43`, `features/settings/keybinding-row.tsx:92`). */
    { label: 'focusBorderOnCard', stateKey: 'app.focusBorder', containerKey: 'panel.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** Monaco `inputOption.activeBorder` / focused input outline against the field it outlines (`monaco/theme.ts:66`). */
    {
        label: 'inputFocusBorder',
        stateKey: 'input.focusBorder',
        containerKey: 'input.background',
        surfaceKey: 'panel.background',
        minDeltaE: STATE_MIN_DISTINCT_DELTA_E,
    },

    /** `focus:bg-menu-item-hover` on `bg-menu-background` (`shared/ui/dropdown-menu.tsx:28,56`, `context-menu.tsx:73,97`, Monaco `menu.selectionBackground` at `monaco/theme.ts:148-151`). */
    { label: 'menuItemHover', stateKey: 'menu.itemHover', containerKey: 'menu.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
    /** `data-[state=open]:bg-modal-item-hover` on `bg-modal-background` — the dialog close button (`shared/ui/dialog.tsx:82`). */
    { label: 'modalItemHover', stateKey: 'modal.itemHover', containerKey: 'modal.background', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },

    /**
     * `bg-scrollbar-thumb` against the track it slides in (`shared/scroll/overlay-scrollbar.tsx:35`).
     * Inert for converted themes — `mapping-tables.ts` derives `scrollbar.track` as the literal
     * `transparent`, and a non-hex container is skipped rather than guessed at — but a hand-authored
     * or user-saved theme (`save_theme`) can set an opaque track, which is what this guards.
     */
    { label: 'scrollbarThumb', stateKey: 'scrollbar.thumb', containerKey: 'scrollbar.track', minDeltaE: STATE_MIN_DISTINCT_DELTA_E },
]
