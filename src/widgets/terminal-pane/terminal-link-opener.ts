export type TerminalLinkOpenerDeps = {
    windowOpen: (uri: string) => Window | null
    openExternalUrl: (uri: string) => Promise<unknown>
}

export const openTerminalLink = async (uri: string, deps: TerminalLinkOpenerDeps) => {
    if (deps.windowOpen(uri)) return
    await deps.openExternalUrl(uri)
}
