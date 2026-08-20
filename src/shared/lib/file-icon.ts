import { extractFileExtension } from '@shared/lib/file-extension'

export type FileIconName =
    | 'file'
    | 'file-code'
    | 'component'
    | 'file-json'
    | 'book-text'
    | 'palette'
    | 'globe'
    | 'cog'
    | 'coffee'
    | 'terminal'
    | 'file-cog'
    | 'lock'
    | 'image'
    | 'file-text'
    | 'file-archive'
    | 'package'
    | 'settings-2'
    | 'git-branch'
    | 'git-fork'
    | 'container'
    | 'book-marked'
    | 'scale'
    | 'key'
    | 'folder'
    | 'folder-open'
    | 'folder-code'
    | 'box'
    | 'flask-conical'

export type FileIconSpec = { icon: FileIconName; colorClass: string }

const COLOR = {
    info: 'text-status-info',
    warning: 'text-status-warning',
    error: 'text-status-error',
    success: 'text-status-success',
    renamed: 'text-git-renamed',
    conflicted: 'text-git-conflicted',
    staged: 'text-git-staged',
    neutral: 'text-app-sidebar-icon-default',
} as const

const DEFAULT_FILE_ICON: FileIconSpec = { icon: 'file', colorClass: COLOR.neutral }

const SPECIAL_FILE_NAME_ICON: Record<string, FileIconSpec> = {
    'package.json': { icon: 'package', colorClass: COLOR.error },
    'tsconfig.json': { icon: 'settings-2', colorClass: COLOR.info },
    'cargo.toml': { icon: 'cog', colorClass: COLOR.conflicted },
    'cargo.lock': { icon: 'lock', colorClass: COLOR.neutral },
    '.gitignore': { icon: 'git-branch', colorClass: COLOR.conflicted },
    dockerfile: { icon: 'container', colorClass: COLOR.info },
}

const README_ICON: FileIconSpec = { icon: 'book-marked', colorClass: COLOR.info }
const LICENSE_ICON: FileIconSpec = { icon: 'scale', colorClass: COLOR.warning }
const ENV_ICON: FileIconSpec = { icon: 'key', colorClass: COLOR.warning }

const EXTENSION_ICON: Record<string, FileIconSpec> = {
    ts: { icon: 'file-code', colorClass: COLOR.info },
    tsx: { icon: 'component', colorClass: COLOR.info },
    js: { icon: 'file-code', colorClass: COLOR.warning },
    jsx: { icon: 'component', colorClass: COLOR.warning },
    json: { icon: 'file-json', colorClass: COLOR.neutral },
    md: { icon: 'book-text', colorClass: COLOR.info },
    css: { icon: 'palette', colorClass: COLOR.info },
    html: { icon: 'globe', colorClass: COLOR.conflicted },
    rs: { icon: 'cog', colorClass: COLOR.staged },
    py: { icon: 'file-code', colorClass: COLOR.warning },
    go: { icon: 'file-code', colorClass: COLOR.renamed },
    java: { icon: 'coffee', colorClass: COLOR.error },
    sh: { icon: 'terminal', colorClass: COLOR.success },
    yml: { icon: 'file-cog', colorClass: COLOR.error },
    yaml: { icon: 'file-cog', colorClass: COLOR.error },
    toml: { icon: 'file-cog', colorClass: COLOR.neutral },
    lock: { icon: 'lock', colorClass: COLOR.neutral },
    png: { icon: 'image', colorClass: COLOR.staged },
    jpg: { icon: 'image', colorClass: COLOR.staged },
    jpeg: { icon: 'image', colorClass: COLOR.staged },
    gif: { icon: 'image', colorClass: COLOR.staged },
    webp: { icon: 'image', colorClass: COLOR.staged },
    svg: { icon: 'image', colorClass: COLOR.staged },
    pdf: { icon: 'file-text', colorClass: COLOR.error },
    zip: { icon: 'file-archive', colorClass: COLOR.warning },
}

const DEFAULT_FOLDER_ICON = { closed: 'folder', open: 'folder-open' } as const

const SPECIAL_FOLDER_NAME_ICON: Record<string, FileIconSpec> = {
    src: { icon: 'folder-code', colorClass: COLOR.info },
    node_modules: { icon: 'package', colorClass: COLOR.neutral },
    dist: { icon: 'box', colorClass: COLOR.warning },
    build: { icon: 'box', colorClass: COLOR.warning },
    test: { icon: 'flask-conical', colorClass: COLOR.success },
    tests: { icon: 'flask-conical', colorClass: COLOR.success },
    docs: { icon: 'book-marked', colorClass: COLOR.info },
    public: { icon: 'globe', colorClass: COLOR.renamed },
    assets: { icon: 'image', colorClass: COLOR.staged },
    '.git': { icon: 'git-branch', colorClass: COLOR.conflicted },
    '.github': { icon: 'git-fork', colorClass: COLOR.info },
}

export const resolveFileIcon = (fileName: string): FileIconSpec => {
    const nameLower = fileName.toLowerCase()

    const specialByName = SPECIAL_FILE_NAME_ICON[nameLower]
    if (specialByName) return specialByName

    if (nameLower.startsWith('readme')) return README_ICON
    if (nameLower.startsWith('license') || nameLower.startsWith('licence')) return LICENSE_ICON
    if (nameLower.startsWith('.env')) return ENV_ICON

    const extension = extractFileExtension(nameLower)
    if (!extension) return DEFAULT_FILE_ICON

    return EXTENSION_ICON[extension] ?? DEFAULT_FILE_ICON
}

export const resolveFolderIcon = (folderName: string, expanded: boolean): FileIconSpec => {
    const nameLower = folderName.toLowerCase()
    const special = SPECIAL_FOLDER_NAME_ICON[nameLower]
    if (special) return special

    return { icon: expanded ? DEFAULT_FOLDER_ICON.open : DEFAULT_FOLDER_ICON.closed, colorClass: COLOR.neutral }
}
