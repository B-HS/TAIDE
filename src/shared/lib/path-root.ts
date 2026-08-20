const WINDOWS_SEPARATOR_PATTERN = /\\/g

const DRIVE_LETTER_PATTERN = /^([a-zA-Z]:)(\/.*)?$/

/**
 * Lexically resolves `.`/`..` segments and unifies path separators to `/`, without touching the
 * filesystem — this module runs in the renderer, which has no fs access, only whatever path
 * strings IPC/dialog/monaco hand it (unlike the Rust side's `root_guard.rs::ensure_within_root`,
 * which can afford `std::fs::canonicalize`). A plain `path === root || path.startsWith(`${root}/`)`
 * check has two bugs this fixes: `..` segments in an unresolved path can walk outside `root` while
 * still string-matching its prefix, and a hardcoded `/` join breaks on Windows' backslash-separated
 * paths. Normalizing both sides through this function before comparing ({@link isWithinRoot}) fixes
 * both at once.
 */
const normalizeFsPath = (path: string) => {
    const unified = path.replace(WINDOWS_SEPARATOR_PATTERN, '/')
    const driveMatch = DRIVE_LETTER_PATTERN.exec(unified)
    const anchor = driveMatch ? driveMatch[1] : unified.startsWith('/') ? '/' : ''
    const rest = driveMatch ? (driveMatch[2] ?? '') : unified.slice(anchor.length)

    const segments: string[] = []
    for (const segment of rest.split('/')) {
        if (segment === '' || segment === '.') continue
        if (segment === '..') {
            if (segments.length > 0) segments.pop()
            continue
        }
        segments.push(segment)
    }

    if (anchor === '') return segments.join('/')
    return anchor === '/' ? `/${segments.join('/')}` : `${anchor}/${segments.join('/')}`
}

/** Whether `path` is `root` itself or lexically nested under it, after normalizing both through {@link normalizeFsPath}. */
export const isWithinRoot = (path: string, root: string) => {
    const normalizedPath = normalizeFsPath(path)
    const normalizedRoot = normalizeFsPath(root)
    return normalizedPath === normalizedRoot || normalizedPath.startsWith(`${normalizedRoot}/`)
}
