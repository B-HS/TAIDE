import type { FileSizeTier } from '@shared/api/bindings'

/**
 * Whether `EditorPane`'s render for this file-query state includes `<CodeEditor>`. Mirrors the
 * component's three early returns (`isPending`, `isError`, `tier === 'refused'`) — every other
 * branch (markdown preview split, blame footer, conflict banners) always renders `<CodeEditor>`
 * underneath. Extracted so the render-phase `editor` state adjustment that depends on this
 * decision (docs/acknowledge/2026-08-20-blank-window-hotfix-contract.md §2) has one source of
 * truth shared with a unit test, instead of duplicating the three-branch check inline where it
 * cannot be exercised without mounting monaco.
 */
export const canRenderCodeEditor = (isPending: boolean, isError: boolean, tier: FileSizeTier | undefined) =>
    !isPending && !isError && tier !== 'refused'
