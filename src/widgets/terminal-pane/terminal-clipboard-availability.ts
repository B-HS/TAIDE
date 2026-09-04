type ClipboardCapabilityProbe = {
    hasWriteText: boolean
    hasReadText: boolean
}

/**
 * Whether the terminal's context menu may offer Copy and Paste at all.
 *
 * The async clipboard API is exposed only in secure contexts, and the remote mirror is served over
 * plain LAN HTTP (`docs/features/remote-control.md`) — so `navigator.clipboard` is simply absent
 * there, which is why probing for the two methods is also the secure-context test and no separate
 * `isSecureContext` check is needed. Deciding this up front is what keeps the two entries from
 * being silently dead: an item the user can click that never does anything is worse than a disabled
 * one that says so. Split from {@link probeTerminalClipboardAvailability} the same way
 * `runtime-environment.ts` splits `isRemoteMirrorLabel` from `isRemoteMirrorRuntime`, so the
 * decision itself runs under `bun:test`, where there is no `navigator` at all.
 */
export const resolveTerminalClipboardAvailability = ({ hasWriteText, hasReadText }: ClipboardCapabilityProbe) => ({
    canCopy: hasWriteText,
    canPaste: hasReadText,
})

/** Reads the live browser capabilities and hands them to {@link resolveTerminalClipboardAvailability}. */
export const probeTerminalClipboardAvailability = () =>
    resolveTerminalClipboardAvailability({
        hasWriteText: typeof navigator !== 'undefined' && typeof navigator.clipboard?.writeText === 'function',
        hasReadText: typeof navigator !== 'undefined' && typeof navigator.clipboard?.readText === 'function',
    })
