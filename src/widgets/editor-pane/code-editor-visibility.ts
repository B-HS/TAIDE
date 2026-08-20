import type { FileSizeTier } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'

/**
 * Whether `EditorPane`'s render for this file-query state includes `<CodeEditor>`. Mirrors the
 * component's three early returns (`isPending`, `isError`, `tier === 'refused'`) — every other
 * branch (markdown preview split, blame footer, conflict banners) always renders `<CodeEditor>`
 * underneath. Extracted so this decision has one source of truth shared by the render-phase
 * `editor` state adjustment ({@link resolveEditorStateForRender}), the LSP session attach gate
 * (`use-editor-lsp-integration.ts`), and a unit test — instead of the three-branch check being
 * duplicated inline at each site, where it cannot be exercised without mounting monaco.
 * `tier` accepts `null` as well as `undefined` since callers source it differently: `EditorPane`
 * passes `file?.tier` (`undefined` before the file query resolves), while
 * `useEditorLspIntegration` normalizes it to `null` at its own call site.
 */
export const canRenderCodeEditor = (isPending: boolean, isError: boolean, tier: FileSizeTier | null | undefined) =>
    !isPending && !isError && tier !== 'refused'

/**
 * The `editor` state `EditorPane` should carry out of the CURRENT render, kept consistent with
 * whether that render actually outputs `<CodeEditor>`: the live instance when
 * {@link canRenderCodeEditor} is true, `null` otherwise. See §2 of
 * docs/acknowledge/2026-08-20-blank-window-hotfix-contract.md. Only covers the three
 * `canRenderCodeEditor`-false branches (loading,
 * error, refused tier) — a commit where `canRenderCodeEditor` stays true throughout but
 * `CodeEditor` still unmounts and remounts (a sibling JSX branch flipping element type, e.g. the
 * markdown-preview split) is outside what this function can see, and is instead closed
 * structurally in `editor-pane.tsx`'s JSX (contract §7).
 */
export const resolveEditorStateForRender = (
    editor: monaco.editor.IStandaloneCodeEditor | null,
    isPending: boolean,
    isError: boolean,
    tier: FileSizeTier | null | undefined,
) => (canRenderCodeEditor(isPending, isError, tier) ? editor : null)
