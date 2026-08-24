import type { FC } from 'react'
import { CornerDownLeft } from 'lucide-react'
import type { PaletteLineTarget } from '@shared/lib/command-palette-query'
import { CommandGroup, CommandItem } from '@shared/ui/command'

type CommandPaletteLineGroupProps = {
    lineTarget: PaletteLineTarget | null
    activePath: string | null
    onSelectLine: (target: PaletteLineTarget) => void
}

export const CommandPaletteLineGroup: FC<CommandPaletteLineGroupProps> = ({ lineTarget, activePath, onSelectLine }) => {
    if (!lineTarget || !activePath) return null

    return (
        <CommandGroup>
            <CommandItem onSelect={() => onSelectLine(lineTarget)}>
                <CornerDownLeft className='size-4' />
                <span>{lineTarget.column > 1 ? `${lineTarget.line}:${lineTarget.column}` : `${lineTarget.line}`}</span>
            </CommandItem>
        </CommandGroup>
    )
}
