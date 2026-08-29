import { useSyncExternalStore } from 'react'
import type { QueryClient } from '@tanstack/react-query'
import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query'
import type { ResolvedTheme } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { deleteTheme, getCurrentTheme, getTheme, listThemes, saveTheme } from '@entities/theme/theme.ipc'
import { readSystemTheme } from '@shared/lib/system-appearance'
import { diffThemeValues } from '@shared/lib/theme-draft'
import type { ThemeDraft } from '@shared/lib/theme-draft'
import { createExternalStoreBridge } from '@shared/lib/bridge/external-store-bridge'
import { createFrameCoalescer } from '@shared/lib/frame-coalescer'

export const themeListQueryOptions = () => queryOptions({ queryKey: QUERY_KEY.THEME.LIST, queryFn: listThemes })

export const currentThemeQueryOptions = () =>
    queryOptions({ queryKey: QUERY_KEY.THEME.CURRENT, queryFn: () => getCurrentTheme(readSystemTheme()), staleTime: Infinity })

export const themeQueryOptions = (themeId: string) => queryOptions({ queryKey: QUERY_KEY.THEME.DETAIL(themeId), queryFn: () => getTheme(themeId) })

export const useSaveTheme = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: saveTheme,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL }),
    })
}

export const useDeleteTheme = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: deleteTheme,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL }),
    })
}

/**
 * `draft.metadata.tokenColors` wins over the base theme's: a draft only carries its own rules when
 * they actually differ from what `extends` would resolve to (`resolveThemeDraftMetadata`), and for
 * an imported `.vsix` theme those rules *are* its highlighting — previewing it through the builtin
 * base's rules (which are always absent) made the whole window fall back to the coarse
 * syntax-derived rules the moment the editor opened, so the preview showed a theme the user had not
 * asked for and Save could not reproduce (audit §4-B B6).
 */
const toResolvedThemeFromDraft = (draft: ThemeDraft, baseTokenColors: ResolvedTheme['tokenColors']): ResolvedTheme => ({
    id: draft.id,
    name: draft.name,
    type: draft.themeType,
    colors: draft.current.colors,
    syntax: draft.current.syntax,
    terminal: draft.current.terminal,
    tokenColors: draft.metadata.tokenColors ?? baseTokenColors,
    syntaxOverrides: Object.keys(diffThemeValues(draft.base, draft.current).syntax),
    warnings: [],
    author: draft.metadata.author,
    license: draft.metadata.license,
    source: draft.metadata.source,
})

/**
 * Holds the theme editor's live preview override outside the query cache (contract F1#11≡R5#10):
 * a draft-in-progress is client-only UI state, not a server fact, so it must never overwrite
 * `QUERY_KEY.THEME.CURRENT` — that key is reserved for what `theme_get_current` actually resolved.
 * `ThemeProvider` reads this alongside {@link currentThemeQueryOptions} and prefers it when set.
 */
const themePreviewStore = createExternalStoreBridge<ResolvedTheme | null>(null)

export const useThemePreviewValue = () => useSyncExternalStore(themePreviewStore.subscribe, themePreviewStore.getSnapshot)

type ThemePreviewPushPayload = { draft: ThemeDraft; queryClient: QueryClient }

/**
 * Coalesces `setPreview` calls to at most one full re-resolve + store publish per animation frame,
 * keeping only the most recently pushed draft — a color-picker drag calls `setPreview` once per
 * `pointermove`, and each call re-runs `toResolvedThemeFromDraft` (a full theme object rebuild) and
 * publishes to {@link themePreviewStore}, which fans out to every subscriber (`ThemeProvider`'s CSS
 * variable + shiki + window-appearance re-apply). Applying every intermediate draft floods the main
 * thread the same way the per-move native window-appearance IPC did before it was gated (contract
 * d-45 §0, §1#2).
 *
 * Lives at module scope, next to {@link themePreviewStore}, instead of per-`useThemePreview()`-call
 * (contract d-45 F-04): the store this guards is already a module singleton, so a coalescer scoped to
 * one caller cannot protect a second caller's still-pending frame — that second caller's own
 * `clearPreview` would only cancel its own coalescer, leaving the first caller's already-pushed frame
 * free to flush a stale draft back into the shared store after the second caller believed it had
 * cleared it (the exact flood-reversal shape contract d-45 §1's review question 2 asks about, just one
 * hook instance removed). Matching this coalescer's lifetime to the store's — both live for the whole
 * process — closes that gap no matter how many components call `useThemePreview()`. `queryClient`
 * travels inside each pushed payload rather than being captured once at creation, since module scope
 * has no hook to read it from.
 */
const themePreviewCoalescer = createFrameCoalescer<ThemePreviewPushPayload>(({ draft, queryClient }) => {
    const baseTheme = queryClient.getQueryData<ResolvedTheme>(QUERY_KEY.THEME.DETAIL(draft.extendsId))
    themePreviewStore.setValue(toResolvedThemeFromDraft(draft, baseTheme?.tokenColors))
})

/**
 * Thin per-call wrapper around the module-singleton {@link themePreviewCoalescer} and
 * {@link themePreviewStore} — it holds no state of its own, only `useQueryClient()` to fill in each
 * push's payload. `clearPreview` cancels any not-yet-flushed draft before clearing the store, so a
 * pending frame from a drag that just ended can never re-publish a stale preview after the editor
 * closed (contract d-45 §1 review question 2).
 */
export const useThemePreview = () => {
    const queryClient = useQueryClient()

    return {
        setPreview: (draft: ThemeDraft) => themePreviewCoalescer.push({ draft, queryClient }),
        clearPreview: () => {
            themePreviewCoalescer.cancel()
            themePreviewStore.setValue(null)
        },
    }
}
