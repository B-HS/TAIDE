import type { FC } from 'react'
import { Check, GitBranch } from 'lucide-react'
import type { GitBranch as GitBranchInfo } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { CommandGroup, CommandItem } from '@shared/ui/command'

type BranchGroupProps = {
    heading: string
    branches: GitBranchInfo[]
    filter: string
    onSelect: (name: string) => void
}

export const BranchGroup: FC<BranchGroupProps> = ({ heading, branches, filter, onSelect }) => {
    const visible = branches.filter((branch) => branch.name.toLowerCase().includes(filter.toLowerCase()))
    if (visible.length === 0) return null

    return (
        <CommandGroup heading={heading}>
            {visible.map((branch) => (
                <CommandItem key={branch.name} onSelect={() => onSelect(branch.name)}>
                    <GitBranch className={cn('size-4', branch.isHead && 'text-app-accent')} />
                    <span className='truncate'>{branch.name}</span>
                    {branch.isHead && <Check className='text-app-accent ml-auto size-4 shrink-0' />}
                </CommandItem>
            ))}
        </CommandGroup>
    )
}
