export const extractFileExtension = (fileName: string) => {
    const dotIndex = fileName.lastIndexOf('.')
    return dotIndex <= 0 ? null : fileName.slice(dotIndex + 1).toLowerCase()
}
