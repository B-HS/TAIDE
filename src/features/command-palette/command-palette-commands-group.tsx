import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { Terminal } from 'lucide-react'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import { isCommandRunnable } from '@shared/lib/command-registry'
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

/**
 * Renders `FuzzyRankedItem.label` rather than re-running `formatCategorizedLabel`: that label is the
 * NFC-normalized string `fuzzyFilter` matched, so it is the only string `match.indices` line up with.
 */
export const CommandPaletteCommandsGroup: FC<CommandPaletteCommandsGroupProps> = ({ commands, keybindingRows, commandContext, onRunCommand }) => {
    const { t } = useTranslation()

    return (
        <CommandGroup heading={t('palette.commands')}>
            {commands.map(({ item, match, label }) => {
                const keybindingRow = findKeybindingRowById(keybindingRows, item.keymapId ?? item.id)
                const runnable = isCommandRunnable(item, commandContext)
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
