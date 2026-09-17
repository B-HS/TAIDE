import type { PaneNode, TabId, TabKind } from '@shared/api/bindings'

type PaneLeaf = Extract<PaneNode, { node: 'leaf' }>

/**
 * Tab kinds `features/editor/code-editor.tsx` registers into `entities/editor/editor-instance-registry`
 * (via its `registryTabId` prop) — the only kinds a monaco `CodeEditor` instance backs, and so the
 * only kinds `⌘S`/`taide.*` monaco-action dispatch (`editor-area.tsx`'s `saveActiveTab`/
 * `runMonacoAction`) can ever reach through `getEditorInstance(tabId)`. `settings`/`terminal`/`diff`/
 * `claudeDiff`/`searchEditor`/`welcome` render their own widgets instead and correctly stay no-ops
 * here.
 *
 * `untitled` joined the set in d-66 (audit info `persistence-5`), together with the
 * `registryTabId={tabId}` `untitled-pane.tsx` now passes. Until then its `<CodeEditor>` registered
 * nothing, so `⌘S` on a focused `untitled` tab no-op'd for the same unregistered-instance reason
 * `appFile` did before d-42 — which left that pane's `onSave={() => void handleSaveAs()}` (the
 * "save as…" dialog, the only way an untitled buffer reaches disk) as unreachable dead code.
 */
const SAVE_ROUTABLE_TAB_KINDS: ReadonlySet<TabKind['kind']> = new Set(['file', 'appFile', 'untitled'])

/**
 * Resolves the focused pane's active tab id, but only when its kind is one `⌘S`/monaco-action
 * dispatch can actually reach (see {@link SAVE_ROUTABLE_TAB_KINDS}) — `null` otherwise, so callers
 * (`getFocusedSaveRoutableEditor`, the active-action-ids sync effect) fall through to their existing
 * no-op path for every other tab kind. Extracted from `editor-area.tsx`'s inline `kind.kind === 'file'`
 * check (contract `2026-08-25-d42-e2e-defects-contract.md` §3, item a) so the kind filter has one
 * unit-testable source of truth instead of being duplicated across `getFocusedSaveRoutableTabId`'s
 * two call sites, and so the fix — an `appFile` tab (`settings.json`/prompt override) now qualifies
 * exactly like a regular `file` tab — is asserted directly rather than only indirectly through a full
 * `EditorArea` render.
 */
export const resolveSaveRoutableTabId = (leaf: Pick<PaneLeaf, 'tabs' | 'active'> | null): TabId | null => {
    const activeTab = leaf?.tabs.find((tab) => tab.id === leaf.active)
    return activeTab && SAVE_ROUTABLE_TAB_KINDS.has(activeTab.kind.kind) ? activeTab.id : null
}
