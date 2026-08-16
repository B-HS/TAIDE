import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const extractVsixThemes = (vsixPath: string) => unwrapResult(commands.vsixExtractThemes(vsixPath))

export const importVsixPlugin = (vsixPath: string) => unwrapResult(commands.vsixImportPlugin(vsixPath))
