import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const listSnippetFiles = () => unwrapResult(commands.snippetList())

export const saveSnippetFile = (input: { fileName: string; content: string }) => unwrapResult(commands.snippetSave(input.fileName, input.content))

export const deleteSnippetFile = (fileName: string) => unwrapResult(commands.snippetDelete(fileName))
