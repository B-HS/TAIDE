import type { Settings } from '@shared/api/bindings'

type DiffViewSettingsSource = Pick<Settings, 'editorDiffHideUnchangedRegions' | 'editorDiffShowMoves'>

/**
 * Derives the `DiffView` props that come straight from `Settings` (with their defaults) — the
 * `code-editor-settings` counterpart for the diff surface, shared across every `DiffView` host so
 * the SCM diff pane, the commit diff and the conflict compare dialog cannot drift apart. The
 * conflict dialog lives in `features/`, which never reads a query itself, so `editor-pane` resolves
 * these for it and passes the result down.
 */
export const resolveDiffViewSettingsProps = (settings: DiffViewSettingsSource | undefined) => ({
    hideUnchangedRegions: settings?.editorDiffHideUnchangedRegions ?? false,
    showMoves: settings?.editorDiffShowMoves ?? false,
})
