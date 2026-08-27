/**
 * `ProjectLayout.revision` is a per-project monotonic counter (contract X1#11) bumped by every
 * layout-mutating Rust command — a caller that has already observed revision `N` must ignore any
 * later delivery of a revision `<= N`. Two independent delivery paths need exactly this guard:
 * `app/providers/ipc-sync-provider.tsx`'s `layout:changed` event handler (two mutations completing
 * close together each schedule their own `invalidateQueries` → `getLayout` refetch, and nothing but
 * delivery order guarantees the *newer* revision's refetch is the one that lands last) and
 * `entities/layout/layout.query.ts`'s mutation `onSuccess` handlers (a mutation's own IPC response
 * — e.g. an earlier keystroke's `setTabDirty({dirty:true})` racing a subsequent `⌘S`'s
 * `setTabDirty({dirty:false})` — can resolve out of order the same way, and a raw `setQueryData`
 * with no ordering check would silently clobber the fresher cache entry with the stale one). Both
 * sides read/write the same `QUERY_KEY.LAYOUT.DETAIL` cache, so sharing one staleness rule here
 * (rather than each re-implementing its own revision comparison) keeps them consistent — see
 * `docs/acknowledge/2026-08-25-d42-e2e-defects-contract.md` §3 (item b) for the dirty-dot
 * persistence bug this closes.
 */
export const isStaleLayoutRevision = (lastObservedRevision: number | undefined, incomingRevision: number) =>
    lastObservedRevision !== undefined && incomingRevision <= lastObservedRevision
