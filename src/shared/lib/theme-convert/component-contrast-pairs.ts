export type ComponentContrastPair = {
    label: string
    foregroundKey: string
    backgroundKey: string
    /**
     * The token the background is painted on. Required rather than optional because a background
     * that carries alpha is a normal case in this catalog, not an exception — `button.background`
     * ships `#rrggbbaa` in 9 of the bundled themes, `list.hoverBackground` in 20, and the whole
     * `rose-pine` family sets `tabBar.background`/`tabBar.tabInactiveBackground` to `#00000000`.
     * Reading any of those at their raw RGB measures a color nothing on screen renders (an
     * `#00000000` strip would score as pure black), so every pair names the surface underneath and
     * `contrast.ts` composites before measuring. The surface itself is composited over
     * `app.background`, the one token no bundled theme gives an alpha channel, which is what makes
     * a translucent surface (`darcula`'s `appSidebar.background` `#ffffff1a`) resolve correctly too.
     */
    surfaceKey: string
}

/**
 * Every (text color, surface it is drawn on) pair TAIDE's UI actually renders, derived from the
 * components rather than from the token vocabulary — `global.css`'s `@theme inline` block and its
 * shadcn bridge variables (`--primary`, `--secondary`, `--accent-foreground`, `--muted-foreground`)
 * map each `--taide-*` token to the Tailwind color a component names, and `monaco/theme.ts`'s
 * `MONACO_COLOR_SOURCE` maps the rest onto the embedded editor. Each entry's doc names the file the
 * pairing comes from.
 *
 * Deliberately a separate table from `contrast.ts`'s `CONTRAST_PAIRS` rather than more rows in it.
 * Those 7 pairs carry two hard behavioral commitments — the 5 blocking ones decide whether a VSIX
 * import is rejected (`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §1-a's
 * "기존 5쌍 판정·수리 불변"), and `bundled-theme-contrast.test.ts` asserts the exact violation count
 * of the 2 advisory ones per exempted theme. Rows added to that array would change both. The pairs
 * here are audited by {@link validateComponentContrast} and repaired by
 * {@link repairComponentContrast}, neither of which can reject an import.
 *
 * Two classes of real render pair are deliberately absent, both recorded with their measurements in
 * `docs/theme-system.md` §8.6:
 * - body text (`app.foreground`/`editor.foreground`) on a state overlay or a widget surface — the
 *   only token this table's repair rule could move is the global body color, so a shortfall here is
 *   a statement about the *background* (a state token `state-distinctness-pairs.ts` owns), not
 *   about the text;
 * - axes whose foreground token must serve two differently-colored backgrounds at once
 *   (`button.primaryForeground` on the primary button, the destructive button and the Monaco badge),
 *   which no single value can satisfy in 4 bundled themes and which forces changes of up to ΔE 93
 *   on the primary button in others.
 *
 * This table is the single source of truth for the Rust catalog lint in
 * `src-tauri/src/domain/theme/service.rs`, which replicates it verbatim (plain literals, one object
 * per entry, no computed keys); a label/count comparison test keeps the two from drifting, the same
 * arrangement `state-distinctness-pairs.ts` and `service.rs`'s token-list comparison already use.
 */
export const COMPONENT_CONTRAST_PAIRS: readonly ComponentContrastPair[] = [
    /** Monaco `list.hoverForeground`/`peekViewResult.fileForeground` on `dropdown.listBackground` (`monaco/theme.ts:90,155`) — a list row at rest. */
    { label: 'listRow', foregroundKey: 'list.foreground', backgroundKey: 'list.background', surfaceKey: 'app.background' },
    /** `--accent-foreground` on `--accent` (`global.css`): `focus:text-accent-foreground` menu items and ghost-button hover text. */
    { label: 'listHoverRow', foregroundKey: 'list.foreground', backgroundKey: 'list.hoverBackground', surfaceKey: 'list.background' },

    /** `text-explorer-git-added` on the file tree (`features/explorer/file-tree-row.tsx:26`). */
    { label: 'explorerGitAdded', foregroundKey: 'explorer.gitAdded', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-explorer-git-modified` on the file tree (`features/explorer/file-tree-row.tsx:27`). */
    { label: 'explorerGitModified', foregroundKey: 'explorer.gitModified', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-explorer-git-deleted` on the file tree (`features/explorer/file-tree-row.tsx:28`). */
    { label: 'explorerGitDeleted', foregroundKey: 'explorer.gitDeleted', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-explorer-git-untracked` on the file tree (`features/explorer/file-tree-row.tsx:30`). */
    { label: 'explorerGitUntracked', foregroundKey: 'explorer.gitUntracked', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },

    /** `text-git-added` on a git panel status row (`features/git/status-row-item.tsx:23`, `widgets/git-panel/commit-detail-panel.tsx:36`). */
    { label: 'gitAdded', foregroundKey: 'git.added', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-git-modified` on a git panel status row (`features/git/status-row-item.tsx:22,27`). */
    { label: 'gitModified', foregroundKey: 'git.modified', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-git-deleted` on a git panel status row (`features/git/status-row-item.tsx:24`). */
    { label: 'gitDeleted', foregroundKey: 'git.deleted', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-git-renamed` on a git panel status row and on the file tree (`features/git/status-row-item.tsx:25`, `features/explorer/file-tree-row.tsx:29`). */
    { label: 'gitRenamed', foregroundKey: 'git.renamed', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-git-untracked` on a git panel status row (`features/git/status-row-item.tsx:26`). */
    { label: 'gitUntracked', foregroundKey: 'git.untracked', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-git-conflicted` on a git panel status row and on the file tree (`features/git/status-row-item.tsx:28`, `features/explorer/file-tree-row.tsx:31`). */
    { label: 'gitConflicted', foregroundKey: 'git.conflicted', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-git-staged` on a staged-file icon (`shared/lib/file-icon.ts:42`). */
    { label: 'gitStaged', foregroundKey: 'git.staged', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },

    /** `text-app-sidebar-icon-default` on the status bar strip (`features/window/status-bar.tsx:97`). */
    { label: 'statusBarText', foregroundKey: 'appSidebar.iconDefault', backgroundKey: 'appSidebar.background', surfaceKey: 'app.background' },
    /** `--muted-foreground` on a settings `Card` (`--card` = `panel.background`): every settings description and secondary label. */
    { label: 'panelMutedText', foregroundKey: 'appSidebar.iconDefault', backgroundKey: 'panel.background', surfaceKey: 'app.background' },
    /** `text-app-sidebar-icon-default` on a tree row's directory path (`features/git/status-row-item.tsx:65`, `features/problems/problem-row.tsx`). */
    { label: 'explorerMutedText', foregroundKey: 'appSidebar.iconDefault', backgroundKey: 'explorer.background', surfaceKey: 'app.background' },
    /** `text-app-sidebar-badge` on a settings `Card` — the "not configured" marker (`features/settings/remote-password-row.tsx:36,39`, `ai-provider-token-row.tsx:34`). */
    { label: 'panelBadge', foregroundKey: 'appSidebar.badge', backgroundKey: 'panel.background', surfaceKey: 'app.background' },

    /** `text-status-error` on the status bar strip — the problem count and the chord no-match flash (`features/window/status-bar.tsx:112,147`). */
    { label: 'statusBarError', foregroundKey: 'statusIndicator.error', backgroundKey: 'appSidebar.background', surfaceKey: 'app.background' },
    /** `text-status-warning` on the status bar strip — the pending-chord indicator (`features/window/status-bar.tsx:147`). */
    { label: 'statusBarWarning', foregroundKey: 'statusIndicator.warning', backgroundKey: 'appSidebar.background', surfaceKey: 'app.background' },
    /** `text-status-success` on the status bar strip — LSP and IDE connection state (`features/window/status-bar.tsx:117,128`). */
    { label: 'statusBarSuccess', foregroundKey: 'statusIndicator.success', backgroundKey: 'appSidebar.background', surfaceKey: 'app.background' },

    /** `PROBLEM_SEVERITY_COLOR_CLASS.error` on the problems panel (`features/problems/problem-severity.ts:18`, `problems-panel.tsx:60`). */
    { label: 'problemError', foregroundKey: 'statusIndicator.error', backgroundKey: 'panel.background', surfaceKey: 'app.background' },
    /** `PROBLEM_SEVERITY_COLOR_CLASS.warning`, and every settings warning line (`features/problems/problem-severity.ts:19`, `features/settings/remote-section.tsx:73`). */
    { label: 'problemWarning', foregroundKey: 'statusIndicator.warning', backgroundKey: 'panel.background', surfaceKey: 'app.background' },
    /** `PROBLEM_SEVERITY_COLOR_CLASS.info` on the problems panel (`features/problems/problem-severity.ts:20`). */
    { label: 'problemInfo', foregroundKey: 'statusIndicator.info', backgroundKey: 'panel.background', surfaceKey: 'app.background' },
    /** `text-status-success` on a panel surface — VSIX import and LSP install results (`features/plugin/vsix-import-grammars-section.tsx:26`). */
    { label: 'problemSuccess', foregroundKey: 'statusIndicator.success', backgroundKey: 'panel.background', surfaceKey: 'app.background' },

    /** Monaco `input.foreground` on `input.background` (`monaco/theme.ts:61-62`) — what the user types into the find/replace field. */
    { label: 'inputText', foregroundKey: 'input.foreground', backgroundKey: 'input.background', surfaceKey: 'editor.widgetBackground' },
    /** Monaco `input.placeholderForeground` on the same field (`monaco/theme.ts:64`). */
    { label: 'inputPlaceholder', foregroundKey: 'input.placeholder', backgroundKey: 'input.background', surfaceKey: 'editor.widgetBackground' },

    /** `bg-primary text-primary-foreground` (`shared/ui/button.tsx:12`) — the default button, on a settings `Card` or a dialog. */
    { label: 'buttonPrimary', foregroundKey: 'button.primaryForeground', backgroundKey: 'button.primaryBackground', surfaceKey: 'panel.background' },
    /** `bg-secondary text-secondary-foreground` (`shared/ui/button.tsx:15`), and Monaco's `keybindingLabel` chips (`monaco/theme.ts:158-159`). */
    { label: 'buttonSecondary', foregroundKey: 'button.foreground', backgroundKey: 'button.background', surfaceKey: 'panel.background' },
    /** The same label once the pointer is on it — Monaco `button.secondaryHoverBackground` (`monaco/theme.ts:171`). */
    { label: 'buttonSecondaryHover', foregroundKey: 'button.foreground', backgroundKey: 'button.hoverBackground', surfaceKey: 'panel.background' },

    /** `bg-tab-bar-tab-active-background text-tab-bar-tab-active-foreground` (`features/tab/tab-item.tsx:54`). */
    { label: 'tabActive', foregroundKey: 'tabBar.tabActiveForeground', backgroundKey: 'tabBar.tabActiveBackground', surfaceKey: 'app.background' },
    /** `bg-tab-bar-tab-inactive-background text-tab-bar-tab-inactive-foreground` (`features/tab/tab-item.tsx:55`). */
    {
        label: 'tabInactive',
        foregroundKey: 'tabBar.tabInactiveForeground',
        backgroundKey: 'tabBar.tabInactiveBackground',
        surfaceKey: 'app.background',
    },
    /** `text-tab-bar-preview-foreground` — the italic title of a preview tab, which is the active one (`features/tab/tab-item.tsx:66`). */
    { label: 'tabPreview', foregroundKey: 'tabBar.previewForeground', backgroundKey: 'tabBar.tabActiveBackground', surfaceKey: 'app.background' },

    /** xterm `foreground` on `background` (`shared/lib/xterm-theme.ts:8-9`) — every line of terminal output that does not set an ANSI color. */
    { label: 'terminalText', foregroundKey: 'terminal.foreground', backgroundKey: 'terminal.background', surfaceKey: 'app.background' },

    /** Monaco `editorLink.activeForeground` on the editor canvas (`monaco/theme.ts:192`) — a ctrl-hovered link in source. */
    { label: 'editorLink', foregroundKey: 'terminal.linkForeground', backgroundKey: 'editor.background', surfaceKey: 'app.background' },
    /** Monaco `textLink.foreground` inside hover/suggest widgets (`monaco/theme.ts:51`) — the links in a hover card's markdown. */
    { label: 'editorWidgetLink', foregroundKey: 'terminal.linkForeground', backgroundKey: 'editor.widgetBackground', surfaceKey: 'app.background' },

    /**
     * `text-app-foreground` on `focus:bg-menu-item-hover` (`shared/ui/dropdown-menu.tsx:28,56`,
     * `context-menu.tsx:73,97`, Monaco `menu.selectionForeground` at `monaco/theme.ts:152`) — the
     * hovered menu row. The one axis in this table whose repair moves the *background*: the label is
     * the global body color, and the hover tint is the token that was chosen to sit under it (see
     * `contrast.ts`'s `COMPONENT_CONTRAST_BACKGROUND_SUBSTITUTES`).
     */
    { label: 'menuItemHoverText', foregroundKey: 'app.foreground', backgroundKey: 'menu.itemHover', surfaceKey: 'menu.background' },
]

/**
 * Contrast axes whose foreground is a color the component hard-codes rather than a theme token, so
 * no theme value can move it and no repair exists. Reported for the record only — the catalog gate
 * does not fail on them, exactly as `contrast.ts`'s ANSI advisory does not.
 *
 * `#ffffff` on the destructive button is the deliberate exception to "no raw colors in components"
 * recorded in `docs/theme-system.md` §3: the label has to stay legible on a *red* background
 * whatever the theme, and the theme token that used to be used there
 * (`button.primaryForeground`, which serves the primary button's own background) falls below 3:1 on
 * nearly twice as many bundled themes as white does (d-61 review finding G-1).
 */
export type FixedForegroundContrastPair = {
    label: string
    foregroundValue: string
    backgroundKey: string
    surfaceKey: string
}

export const FIXED_FOREGROUND_CONTRAST_PAIRS: readonly FixedForegroundContrastPair[] = [
    /** `bg-destructive text-white` on a settings `Card` or a dialog (`shared/ui/button.tsx:13`, `--destructive` = `statusIndicator.error`). */
    { label: 'destructiveButtonLabel', foregroundValue: '#ffffff', backgroundKey: 'statusIndicator.error', surfaceKey: 'panel.background' },
]

/**
 * Bundled themes that stay below `MIN_CONTRAST_RATIO` on one axis of
 * {@link COMPONENT_CONTRAST_PAIRS} after repair, keyed `<theme id>:<pair label>` with the measured
 * reason. The catalog gate (`bundled-theme-contrast.test.ts`), the repair script
 * (`scripts/repair-theme-contrast.ts`) and the Rust catalog lint all read this one registry, so an
 * entry can never be stale in one place and live in another; the gate additionally asserts that each
 * listed theme really does violate exactly the listed axis.
 *
 * Both entries are the same shape: ayu's UI foreground already sits at the contrast floor against
 * the plain menu background (3.09:1 dark / 3.32:1 light), so every hover tint that reads as a hover
 * at all (`state-distinctness-pairs.ts`'s `menuItemHover` needs ΔE 2.3) spends the remaining margin.
 * `contrast.ts`'s substitution candidate does not help (`list.hoverBackground` measures 2.62 / 2.83,
 * worse than the 2.72 / 2.88 the themes ship), and the only other lever is `app.foreground` itself —
 * the global body color this table deliberately never moves, which is why the matching
 * `app.foreground` axes are excluded outright (`docs/theme-system.md` §8.6).
 */
export const COMPONENT_CONTRAST_EXEMPTIONS: Record<string, string> = {
    'ayu-dark:menuItemHoverText':
        'app.foreground(#5a6378) vs menu.background(#0f131a) 가 이미 3.09:1 — hover 틴트(#1a2029, ΔE 6.48)가 그 여유를 다 쓴다. 후보 list.hoverBackground 는 2.62 로 더 나쁘다.',
    'ayu-light:menuItemHoverText':
        'app.foreground(#828e9f) vs menu.background(#ffffff) 가 3.32:1 이고 흰색보다 밝은 배경은 없다 — hover 틴트(#edeff1, ΔE 5.78)는 반드시 대비를 낮춘다. 후보 list.hoverBackground 는 2.83 으로 더 나쁘다.',
}
