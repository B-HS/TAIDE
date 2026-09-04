/**
 * Decides whether a clicked anchor must leave the app window instead of navigating it.
 *
 * The app's own webview has no browser chrome — no address bar, no back button — so any
 * navigation to a foreign origin strands the user on a page they cannot leave; rendered
 * markdown is the reliable way to hit that (`marked` emits bare `<a href>` with no
 * `target="_blank"`, and the tauri opener plugin's injected click interceptor only ever
 * looked at `target="_blank"` / Ctrl / Shift clicks).
 *
 * Only absolute `http:`/`https:` URLs pointing at a *different* origin qualify. Same-origin
 * links stay in-app because they are the app's own routes and assets; every other scheme
 * (`mailto:`, `file:`, `javascript:`, `blob:`, `about:`) is left to the platform's own
 * handling, matching the Rust whitelist in `system_open_external_url`, which accepts nothing
 * but http(s). Anything that does not parse as an absolute URL — a relative `href`, an empty
 * one — is in-app by definition and returns `false` rather than throwing.
 */
export const shouldOpenAnchorExternally = (anchorHref: string, appOrigin: string) => {
    try {
        const url = new URL(anchorHref)
        if (url.protocol !== 'http:' && url.protocol !== 'https:') return false
        return url.origin !== appOrigin
    } catch {
        return false
    }
}
