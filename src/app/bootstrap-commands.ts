import { DEFAULT_COMMANDS, registerCommands } from '@shared/lib/command-registry'
import { MONACO_ACTION_COMMANDS } from '@shared/lib/monaco-action-commands'
import { AGENT_CLI_COMMANDS } from '@entities/agent/agent.commands'
import { GIT_COMMANDS } from '@entities/git/git.commands'
import { SYNC_COMMANDS } from '@entities/sync/sync.commands'
import { TASK_COMMANDS } from '@entities/task/task.commands'
import { TERMINAL_COMMANDS } from '@entities/terminal/terminal.commands'

registerCommands(DEFAULT_COMMANDS)
registerCommands(SYNC_COMMANDS)
registerCommands(MONACO_ACTION_COMMANDS)
registerCommands(AGENT_CLI_COMMANDS)
registerCommands(GIT_COMMANDS)
registerCommands(TERMINAL_COMMANDS)
registerCommands(TASK_COMMANDS)
