import { resolve } from 'node:path'

const ROOT = resolve(import.meta.dir, '../..')
const TIMEOUT_MS = 5000
const started = performance.now()
const child = Bun.spawn({
    cmd: [resolve(ROOT, 'experiments/native-shell-spike/target/debug/examples/find-regex-probe'), 'backtracking'],
    cwd: ROOT,
    stdout: 'pipe',
    stderr: 'pipe',
    timeout: TIMEOUT_MS,
    killSignal: 'SIGKILL',
})
const [exitCode, stdout, stderr] = await Promise.all([child.exited, new Response(child.stdout).text(), new Response(child.stderr).text()])
console.info({ timeoutMs: TIMEOUT_MS, elapsedMs: performance.now() - started, exitCode, signal: child.signalCode, stdout, stderr })
