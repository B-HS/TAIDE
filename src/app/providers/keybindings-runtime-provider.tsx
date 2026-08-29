import type { FC, PropsWithChildren } from 'react'
import { lazy, Suspense, useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { parseKeymapOverrides } from '@shared/lib/keymap/keymap'
import { subscribeOpenKeybindingsEditor } from '@shared/lib/keymap/keybindings-bridge'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { applyMonacoKeybindingOverrides } from '@shared/lib/monaco/monaco-keybinding-runtime'
import { settingsQueryOptions } from '@entities/settings/settings.query'

/**
 * Split out of the boot payload (audit §1-1) together with its keybinding catalog and conflict
 * detection. This provider is mounted in every window for the two side effects documented below, so
 * an eagerly imported dialog cost every window the editor's whole subtree at launch even though it
 * is only ever on screen after an explicit ⌘K ⌘S / palette / settings-button open.
 */
const KeybindingsEditor = lazy(async () => ({ default: (await import('@widgets/keybindings-editor/keybindings-editor')).KeybindingsEditor }))

/**
 * `unmounted` until the first open request, then `open`/`closed` forever after. A plain
 * `open` boolean would defer the chunk just as well by rendering the dialog only while true, but it
 * would also unmount the dialog the instant it closes and so cut Radix's close animation short —
 * this third state keeps the dialog mounted once it has been opened at least once.
 */
const KEYBINDINGS_DIALOG_STATE = { UNMOUNTED: 'unmounted', OPEN: 'open', CLOSED: 'closed' } as const

type KeybindingsDialogState = (typeof KEYBINDINGS_DIALOG_STATE)[keyof typeof KEYBINDINGS_DIALOG_STATE]

/**
 * Keeps two `KeybindingsEditor`-owned side effects alive independent of the dialog's own mount
 * state — both used to live directly inside that dialog component, which was only ever rendered in
 * the main window (`app.tsx`), so an auxiliary editor window's own monaco instance never received
 * `settings.keymapOverrides` at all, and pressing the "open keymap editor" shortcut inside an
 * auxiliary window published on that window's own local `keybindings-bridge` to zero subscribers
 * (bridges are per-realm module state, not cross-window). Mounted once at the app root for both the
 * main window and every auxiliary window, this renders `KeybindingsEditor` itself as a controlled
 * dialog so both concerns — applying overrides to *this* window's monaco, and opening *this*
 * window's own dialog on *this* window's own shortcut — now exist wherever the provider is mounted.
 *
 * The ⌘K ⌘S keymap handler lives here too, for the same reason: it used to be registered by
 * `AppShell`, which an auxiliary window never mounts, so the chord resolved (and was swallowed,
 * `preventDefault`ed by `useGlobalKeymap`) against an empty handler map there — the key did
 * nothing at all. Registering it in the provider gives every window exactly one owner of the
 * shortcut. The `keybindings-bridge` subscription stays for the other two entry points that reach
 * this dialog without a keystroke (the palette command and the settings view's button).
 */
export const KeybindingsRuntimeProvider: FC<PropsWithChildren> = ({ children }) => {
    const [dialogState, setDialogState] = useState<KeybindingsDialogState>(KEYBINDINGS_DIALOG_STATE.UNMOUNTED)
    const { data: settings } = useQuery(settingsQueryOptions())

    useGlobalKeymap({ 'open-keybindings-editor': () => setDialogState(KEYBINDINGS_DIALOG_STATE.OPEN) })

    useEffect(() => subscribeOpenKeybindingsEditor(() => setDialogState(KEYBINDINGS_DIALOG_STATE.OPEN)), [])
    useEffect(() => applyMonacoKeybindingOverrides(parseKeymapOverrides(settings?.keymapOverrides ?? null)), [settings?.keymapOverrides])

    return (
        <>
            {children}
            {dialogState !== KEYBINDINGS_DIALOG_STATE.UNMOUNTED && (
                <Suspense fallback={null}>
                    <KeybindingsEditor
                        open={dialogState === KEYBINDINGS_DIALOG_STATE.OPEN}
                        onOpenChange={(next) => setDialogState(next ? KEYBINDINGS_DIALOG_STATE.OPEN : KEYBINDINGS_DIALOG_STATE.CLOSED)}
                    />
                </Suspense>
            )}
        </>
    )
}
