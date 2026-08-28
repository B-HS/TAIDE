import { installErrorLogForwarding } from './error-log-forwarding'

/**
 * Side-effect-only entry point for {@link installErrorLogForwarding} — imported for its module
 * evaluation, not any export. `main.tsx` imports this as its very first line so the forwarder is
 * wired before `@app/app`'s module graph (and everything it transitively imports) ever evaluates: a
 * call placed further down `main.tsx`, after those imports, would let a module-evaluation-time throw
 * among them go unlogged — the exact class of silent release-build failure
 * {@link installErrorLogForwarding} exists to catch (its own doc comment, `d-48 §0`).
 */
installErrorLogForwarding()
