export const toRelativePath = (root: string, path: string) => {
    const normalizedRoot = root.endsWith('/') ? root : `${root}/`
    return path.startsWith(normalizedRoot) ? path.slice(normalizedRoot.length) : path
}
