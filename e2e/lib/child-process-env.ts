import { LOGIN_PASSWORD_ENV_VAR } from './constants'

/**
 * Builds an env object for spawning `git`/`pgrep`/`lsof` (`node:child_process`) that omits
 * {@link LOGIN_PASSWORD_ENV_VAR} — none of these child processes need the REMOTE password, and
 * inheriting it via a bare `{ ...process.env }` spread would propagate a secret to processes that
 * have no use for it.
 */
export const buildChildProcessEnv = (extra: Record<string, string> = {}) =>
    Object.fromEntries(Object.entries({ ...process.env, ...extra }).filter(([key]) => key !== LOGIN_PASSWORD_ENV_VAR))
