import { commands } from '@shared/api/bindings'
import type { OpenProjectInSlotRequest, ShellSlotId, WindowChromePatch } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const getShellState = () => unwrapResult(commands.sessionGetShellState())

export const focusShellSlot = (slotId: ShellSlotId) => unwrapResult(commands.sessionFocusShellSlot(slotId))

export const setShellSlotSizes = (path: number[], sizes: number[]) => unwrapResult(commands.sessionSetShellSlotSizes(path, sizes))

export const setWindowChrome = (patch: WindowChromePatch) => unwrapResult(commands.sessionSetWindowChrome(patch))

export const closeShellSlot = (slotId: ShellSlotId) => unwrapResult(commands.shellSlotClose(slotId))

export const openProjectInSlot = (request: OpenProjectInSlotRequest) => unwrapResult(commands.projectOpenInSlot(request))

/** `null` on every axis means "change nothing", matching `emptySettingsPatch`'s convention for the other patch-shaped command. */
export const emptyWindowChromePatch = (): WindowChromePatch => ({ zen: null, sidebarRailCollapsed: null })
