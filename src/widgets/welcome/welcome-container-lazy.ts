import { lazy } from 'react'

/**
 * `WelcomeContainer` is rendered from two places — `app-shell.tsx` (the no-project-open screen) and
 * `pane-node-view.tsx` (the `welcome` tab kind) — so the `lazy()` wrapper lives here instead of
 * being created twice. Two separate `lazy()` calls would share the underlying chunk but not the
 * lazy component's own load state, so each call site would re-suspend independently the first time
 * it rendered.
 *
 * Split out of the boot payload (audit §1-1) because a session that opens straight into a project
 * with restored tabs never renders it at all.
 */
export const WelcomeContainerLazy = lazy(async () => ({ default: (await import('@widgets/welcome/welcome-container')).WelcomeContainer }))
