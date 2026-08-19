import type { FC, PropsWithChildren } from 'react'
import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { parseKeymapOverrides } from '@shared/lib/keymap'
import { subscribeOpenKeybindingsEditor } from '@shared/lib/keybindings-bridge'
import { applyMonacoKeybindingOverrides } from '@shared/lib/monaco-keybinding-runtime'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { KeybindingsEditor } from '@widgets/keybindings-editor/keybindings-editor'

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
 */
export const KeybindingsRuntimeProvider: FC<PropsWithChildren> = ({ children }) => {
    const [open, setOpen] = useState(false)
    const { data: settings } = useQuery(settingsQueryOptions())

    useEffect(() => subscribeOpenKeybindingsEditor(() => setOpen(true)), [])
    useEffect(() => applyMonacoKeybindingOverrides(parseKeymapOverrides(settings?.keymapOverrides ?? null)), [settings?.keymapOverrides])

    return (
        <>
            {children}
            <KeybindingsEditor open={open} onOpenChange={setOpen} />
        </>
    )
}
