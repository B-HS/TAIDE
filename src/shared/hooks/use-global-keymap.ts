import { useSyncExternalStore } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import type { Settings } from '@shared/api/bindings'
import type { KeymapActionId } from '@shared/lib/keymap'
import { APP_KEYMAP, MONACO_CHORD_PREFIX_KEY, applyKeymapOverrides, parseKeymapOverrides } from '@shared/lib/keymap'
import {
    armKeymapMonacoDeferral,
    consumeKeymapMonacoDeferral,
    enterKeymapChordPending,
    getKeymapChordDispatchSnapshot,
    notifyKeymapChordNoMatch,
    resolveKeymapChordPending,
} from '@shared/lib/keymap-chord-store'
import { decideKeymapDispatch } from '@shared/lib/keymap-dispatch'
import { useKeydownCapture } from '@shared/hooks/use-keydown-capture'
import { deriveMonacoChordPrefixes } from '@shared/lib/monaco-keybinding'
import { IS_MAC } from '@shared/constants/platform'
import { QUERY_KEY } from '@shared/constants/query-key'

export type KeymapHandlers = Partial<Record<KeymapActionId, () => void>>

/**
 * Reads keymap overrides straight off the TanStack Query cache by key (`useQueryClient` +
 * `useSyncExternalStore`) instead of importing `entities/settings/settings.query.ts` — `shared`
 * may not import `entities` (FSD). Every current `useGlobalKeymap` consumer widget already runs
 * `useQuery(settingsQueryOptions())` for its own purposes, so the cache is already populated by
 * the time this reads it; this hook never issues its own fetch, only reads whatever is cached.
 * Returns the raw JSON string (a primitive), not the parsed override array, so the
 * `useSyncExternalStore` snapshot stays referentially stable across cache writes unrelated to
 * `keymapOverrides` (the query cache subscription fires on *any* cache event, not just this key).
 */
const useKeymapOverridesJson = () => {
    const queryClient = useQueryClient()
    return useSyncExternalStore(
        (onStoreChange) => queryClient.getQueryCache().subscribe(onStoreChange),
        () => queryClient.getQueryData<Settings>(QUERY_KEY.SETTINGS.CURRENT)?.keymapOverrides ?? null,
    )
}

/**
 * Dispatches `APP_KEYMAP` (with the user's overrides applied) to `handlers`, including chord
 * (two-stage) entries and `when`-gated entries — see `keymap-dispatch.ts` for the state machine
 * and `keymap-chord-store.ts` for the cross-listener chord/monaco-deferral state it shares with
 * every other `useGlobalKeymap` call site. Each side-effect branch below is intentionally thin:
 * all the *decision* logic lives in the pure, unit-tested `decideKeymapDispatch`.
 *
 * `handleKeyDown` is a plain function, not wrapped in its own `useEffectEvent` — `entries`/
 * `handlers` staying fresh across renders is already guaranteed one level down, by
 * `useKeydownCapture`'s own Effect Event closing over whatever `handler` it was called with on
 * the latest render (the same reason every existing `useGlobalKeymap` consumer already passes a
 * plain new handlers object literal each render). Wrapping here too would both be redundant and
 * violate `react-hooks/rules-of-hooks` — Effect Event values may only be *called* from an effect/
 * another Effect Event in the same component, not handed to another hook as a value.
 */
export const useGlobalKeymap = (handlers: KeymapHandlers) => {
    const overridesJson = useKeymapOverridesJson()
    const overrides = parseKeymapOverrides(overridesJson)
    const entries = applyKeymapOverrides(APP_KEYMAP, overrides)
    const monacoChordPrefixes = [MONACO_CHORD_PREFIX_KEY, ...deriveMonacoChordPrefixes(overrides)]

    const handleKeyDown = (event: KeyboardEvent) => {
        const action = decideKeymapDispatch(event, entries, getKeymapChordDispatchSnapshot(event), IS_MAC, undefined, monacoChordPrefixes)

        switch (action.type) {
            case 'ignore-modifier-only':
            case 'none':
                return
            case 'defer-to-monaco':
                consumeKeymapMonacoDeferral()
                return
            case 'observe-monaco-chord-prefix':
                armKeymapMonacoDeferral()
                return
            case 'enter-chord':
                event.preventDefault()
                event.stopPropagation()
                enterKeymapChordPending(action.prefix, action.entryId)
                return
            case 'resolve-chord-no-match':
                event.preventDefault()
                event.stopPropagation()
                resolveKeymapChordPending()
                notifyKeymapChordNoMatch()
                return
            case 'resolve-chord-matched':
                event.preventDefault()
                event.stopPropagation()
                resolveKeymapChordPending()
                handlers[action.entryId]?.()
                return
            case 'dispatch': {
                const handler = handlers[action.entryId]
                if (!handler) return
                event.preventDefault()
                event.stopPropagation()
                handler()
                return
            }
        }
    }

    useKeydownCapture(handleKeyDown)
}
