import { error as logErrorToFile, warn as logWarnToFile } from '@tauri-apps/plugin-log'

type ForwardedLogLevel = 'error' | 'warn'

let isInstalled = false
let isForwarding = false

/**
 * Renders one `console.error`/`console.warn` argument (or a caught `error`/`unhandledrejection`
 * value) into a single line safe to hand to `@tauri-apps/plugin-log` — an `Error` keeps its
 * `message` + `stack`, a plain object goes through `JSON.stringify` (a circular or BigInt-bearing
 * object throws and falls back to `String()` rather than escaping the logging path itself, and an
 * `undefined`/function/symbol value that `JSON.stringify` silently turns into `undefined` instead
 * of throwing falls back the same way), and everything else stringifies as-is. Exported separately
 * from {@link installErrorLogForwarding} so it can run under `bun:test`, which has no
 * `window`/`console` DOM environment (the same constraint `window-context.ts` documents for
 * `window`).
 */
export const serializeLogArgument = (value: unknown) => {
    if (value instanceof Error) return value.stack ? `${value.message}\n${value.stack}` : value.message
    if (typeof value === 'string') return value
    try {
        const json = JSON.stringify(value)
        return json === undefined ? String(value) : json
    } catch {
        return String(value)
    }
}

export const serializeLogArguments = (args: unknown[]) => args.map(serializeLogArgument).join(' ')

const forwardToFileLog = (level: ForwardedLogLevel, message: string) => {
    if (isForwarding) return
    isForwarding = true
    try {
        const send = level === 'error' ? logErrorToFile : logWarnToFile
        void send(message).catch(() => {})
    } finally {
        isForwarding = false
    }
}

/**
 * Installs a one-way mirror from browser-side error signals to the desktop app's on-disk log file
 * (`@tauri-apps/plugin-log`, already registered on the Rust side — `src-tauri/src/lib.rs`'s
 * `log_plugin` plus the `main` capability's `log:default`), so a release build's silent failures
 * (contract d-48 §0 — no devtools, `console.error` alone went nowhere) leave a trail behind.
 *
 * Wraps `console.error`/`console.warn` in place — the original still runs first, so a dev build's
 * devtools console behaves exactly as before — and listens for `window`'s `error`/
 * `unhandledrejection`, which `console.error` alone never observes (an exception that unwinds past
 * every try/catch, or a rejected promise nobody attached a `.catch` to).
 *
 * `isForwarding` blocks synchronous re-entrancy: a console call made while building the forwarded
 * message itself (an exotic `toString`/`toJSON`, or `plugin-log`'s own `invoke` path logging on the
 * way out) would otherwise recurse forever. The forwarded call's own promise is always given a
 * no-op `.catch`, so a `plugin-log` IPC failure never surfaces as an `unhandledrejection` that would
 * loop back into this same forwarder.
 *
 * `targetConsole` defaults to the global `console` when a real `window` exists and is a no-op
 * otherwise (SSR has none; neither does `bun:test`, the same constraint {@link serializeLogArgument}
 * documents) — the production call site (`error-log-forwarding-install.ts`) always calls with zero
 * arguments. Passing an explicit console-like object bypasses that no-op for the console-wrapping
 * half only, so `error-log-forwarding.test.ts` can verify the passthrough/re-entrancy/
 * no-unhandledrejection properties above without a real `window`; the `window` event listeners below
 * still require a real `window` and stay unwired either way when one is absent.
 *
 * Call once from the app entry point (`main.tsx`, via `error-log-forwarding-install.ts`) — a second
 * call is a no-op regardless of `targetConsole`.
 */
export const installErrorLogForwarding = (targetConsole?: Pick<Console, 'error' | 'warn'>) => {
    const resolvedConsole = targetConsole ?? (typeof window === 'undefined' ? null : console)
    if (!resolvedConsole) return
    if (isInstalled) return
    isInstalled = true

    const originalConsoleError = resolvedConsole.error.bind(resolvedConsole)
    const originalConsoleWarn = resolvedConsole.warn.bind(resolvedConsole)

    resolvedConsole.error = (...args: unknown[]) => {
        originalConsoleError(...args)
        forwardToFileLog('error', serializeLogArguments(args))
    }
    resolvedConsole.warn = (...args: unknown[]) => {
        originalConsoleWarn(...args)
        forwardToFileLog('warn', serializeLogArguments(args))
    }

    if (typeof window === 'undefined') return
    window.addEventListener('error', (event) => forwardToFileLog('error', serializeLogArgument(event.error ?? event.message)))
    window.addEventListener('unhandledrejection', (event) => forwardToFileLog('error', serializeLogArgument(event.reason)))
}
