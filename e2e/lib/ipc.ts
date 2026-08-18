import type { Page } from '@playwright/test'

type TauriInternalsBridge = {
    invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>
}

/**
 * Calls a Tauri command through `window.__TAURI_INTERNALS__.invoke` — the same bridge the
 * remote-control webview's own shim installs (it forwards to the app over WebSocket; see
 * `src/shared/lib/remote/tauri-internals-shim.ts`). This is the harness's **verification-only**
 * oracle.
 *
 * Isolation contract: every spec drives the app through the UI (keyboard/mouse) and reaches for
 * this helper only to assert outcomes afterward. Using it to *manipulate* state instead would make
 * a test self-fulfilling — it would stop exercising the UI path it exists to verify. The one
 * accepted exception is reading current state to build a realistic next UI input (e.g. fetching
 * `settings_get` before typing a new `settings.json` body) — that is a read, not a state mutation.
 *
 * The command's success payload is returned as-is; a Rust-side `AppError` rejection propagates as a
 * rejected promise from this function. Callers provide the expected shape via the type parameter —
 * this crosses the browser/Node boundary the same way this codebase's own `clientFetch<T>`
 * convention does, since the auto-generated `src/shared/api/bindings.ts` contract is intentionally
 * not imported here (kept decoupled from ongoing frontend/Rust work outside this harness's
 * ownership).
 */
export const invokeIpc = async <T>(page: Page, command: string, args?: Record<string, unknown>): Promise<T> => {
    const result = await page.evaluate(
        async ({ command: invokedCommand, args: invokedArgs }) => {
            const bridge = (window as unknown as { __TAURI_INTERNALS__?: TauriInternalsBridge }).__TAURI_INTERNALS__
            if (!bridge) throw new Error('window.__TAURI_INTERNALS__ 이 없습니다 — remote 웹뷰 셸이 아직 로드되지 않았습니다.')
            return bridge.invoke(invokedCommand, invokedArgs)
        },
        { command, args },
    )
    return result as T
}
