import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { Terminal } from 'lucide-react'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import { formatCategorizedLabel, isCommandRunnable } from '@shared/lib/command-registry'
import type { KeybindingRow } from '@shared/lib/keymap/keybinding-catalog'
import { findKeybindingRowById } from '@shared/lib/keymap/keybinding-catalog'
import { formatKeymapShortcut } from '@shared/lib/keymap/keymap'
import type { FuzzyRankedItem } from '@shared/lib/fuzzy-match'
import { CommandGroup, CommandItem, CommandShortcut } from '@shared/ui/command'
import { HighlightedText } from '@features/command-palette/highlighted-text'

type CommandPaletteCommandsGroupProps = {
    commands: FuzzyRankedItem<AppCommand>[]
    keybindingRows: KeybindingRow[]
    commandContext: CommandContext
    onRunCommand: (command: AppCommand) => void
}

export const CommandPaletteCommandsGroup: FC<CommandPaletteCommandsGroupProps> = ({ commands, keybindingRows, commandContext, onRunCommand }) => {
    const { t } = useTranslation()

    return (
        <CommandGroup heading={t('palette.commands')}>
            {commands.map(({ item, match }) => {
                const keybindingRow = findKeybindingRowById(keybindingRows, item.keymapId ?? item.id)
                const runnable = isCommandRunnable(item, commandContext)
                const label = formatCategorizedLabel(t, item.categoryKey, item.titleKey, item.titleDefaultValue)
                return (
                    <CommandItem key={item.id} disabled={!runnable} onSelect={() => onRunCommand(item)}>
                        <Terminal className='size-4' />
                        <span>
                            <HighlightedText text={label} indices={match.indices} />
                        </span>
                        {keybindingRow?.key && <CommandShortcut>{formatKeymapShortcut(keybindingRow)}</CommandShortcut>}
                        {!keybindingRow?.key && keybindingRow?.defaultBindingLabel && (
                            <CommandShortcut>{keybindingRow.defaultBindingLabel}</CommandShortcut>
                        )}
                    </CommandItem>
                )
            })}
        </CommandGroup>
    )
}
