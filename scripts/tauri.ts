import { join } from 'node:path'

const REPO_ROOT = join(import.meta.dir, '..')
const BUNDLE_CONFIG_PATH = join('src-tauri', 'tauri.bundle.conf.json')
const DEV_CONFIG_PATH = join('src-tauri', 'tauri.dev.conf.json')
const BUILD_SUBCOMMANDS = new Set(['build', 'bundle'])

const run = (command: string[]) => Bun.spawnSync({ cmd: command, cwd: REPO_ROOT, stdout: 'inherit', stderr: 'inherit' })

const exitWith = (result: ReturnType<typeof run>) => process.exit(result.success ? 0 : (result.exitCode ?? 1))

const args = process.argv.slice(2)
const isBuild = BUILD_SUBCOMMANDS.has(args[0])

/**
 * `tauri dev` alone carries the `net.gumyo.taide.dev` identifier overlay (contract d-49, renamed
 * from `dev.taide.app.dev` under contract 2026-08-29-bundle-identifier-rename) so the dev
 * instance's `app_data_dir`/log dir/keyring service/window-state cache never collides with an
 * installed release build's (identifier `net.gumyo.taide`, `tauri.conf.json`) — every other
 * subcommand (`info`, `icon`, …) runs unmodified.
 */
if (args[0] === 'dev') exitWith(run(['tauri', ...args, '--config', DEV_CONFIG_PATH]))

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
