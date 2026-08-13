import { mkdir, copyFile } from 'node:fs/promises'
import { join } from 'node:path'

const REPO_ROOT = join(import.meta.dir, '..')
const SRC_TAURI_DIR = join(REPO_ROOT, 'src-tauri')
const BINARIES_DIR = join(SRC_TAURI_DIR, 'binaries')
const CLI_PACKAGE_NAME = 'taide-cli'
const CLI_BIN_NAME = 'taide-cli'

const runCommand = (command: string[], cwd: string) => {
    const result = Bun.spawnSync({ cmd: command, cwd, stdout: 'inherit', stderr: 'inherit' })
    if (!result.success) {
        console.error(`sidecar build failed: ${command.join(' ')} exited with code ${result.exitCode}`)
        process.exit(result.exitCode ?? 1)
    }
}

const resolveHostTriple = () => {
    const result = Bun.spawnSync({ cmd: ['rustc', '-vV'], cwd: REPO_ROOT, stdout: 'pipe', stderr: 'inherit' })
    if (!result.success) {
        console.error('sidecar build failed: `rustc -vV` did not run (is rustup/cargo on PATH?)')
        process.exit(result.exitCode ?? 1)
    }
    const output = result.stdout.toString()
    const hostLine = output.split('\n').find((line) => line.startsWith('host: '))
    if (!hostLine) {
        console.error(`sidecar build failed: could not find "host:" in \`rustc -vV\` output:\n${output}`)
        process.exit(1)
    }
    return hostLine.slice('host: '.length).trim()
}

const main = async () => {
    const hostTriple = resolveHostTriple()

    runCommand(['cargo', 'build', '-p', CLI_PACKAGE_NAME, '--release'], REPO_ROOT)

    const builtBinary = join(REPO_ROOT, 'target', 'release', CLI_BIN_NAME)
    const sidecarBinary = join(BINARIES_DIR, `${CLI_BIN_NAME}-${hostTriple}`)

    await mkdir(BINARIES_DIR, { recursive: true })
    await copyFile(builtBinary, sidecarBinary)

    console.log(`sidecar ready: ${sidecarBinary}`)
}

await main()
