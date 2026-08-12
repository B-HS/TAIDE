import { DEFAULT_COMMANDS, registerCommands } from '@shared/lib/command-registry'
import { SYNC_COMMANDS } from '@entities/sync/sync.commands'

registerCommands(DEFAULT_COMMANDS)
registerCommands(SYNC_COMMANDS)
