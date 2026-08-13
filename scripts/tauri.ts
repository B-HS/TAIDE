import { join } from 'node:path'

const REPO_ROOT = join(import.meta.dir, '..')
const BUNDLE_CONFIG_PATH = join('src-tauri', 'tauri.bundle.conf.json')
const BUILD_SUBCOMMANDS = new Set(['build', 'bundle'])

const run = (command: string[]) => Bun.spawnSync({ cmd: command, cwd: REPO_ROOT, stdout: 'inherit', stderr: 'inherit' })

const exitWith = (result: ReturnType<typeof run>) => process.exit(result.success ? 0 : (result.exitCode ?? 1))

const args = process.argv.slice(2)
const isBuild = BUILD_SUBCOMMANDS.has(args[0])

if (!isBuild) exitWith(run(['tauri', ...args]))

/**
 * `tauri build`/`tauri bundle` must carry the `externalBin`-bearing bundle config so the
 * `taide-cli` sidecar ships in the release artifact, but that same config must never apply to
 * `tauri dev` — `tauri_build::build()` runs on every `cargo build` (dev included) and aborts if
 * an `externalBin` source file is missing, which it always is before `cli:sidecar` has run.
 */
const sidecarResult = run(['bun', 'run', 'cli:sidecar'])
if (!sidecarResult.success) exitWith(sidecarResult)

exitWith(run(['tauri', ...args, '--config', BUNDLE_CONFIG_PATH]))
