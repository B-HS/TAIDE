const PATH_SEPARATOR = '/'

export const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

export const joinPath = (dir: string, name: string) => `${dir.endsWith(PATH_SEPARATOR) ? dir.slice(0, -1) : dir}${PATH_SEPARATOR}${name}`
