import type { FC } from 'react'
import { useState } from 'react'
import { GitBranch, Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { GitBranch as GitBranchInfo } from '@shared/api/bindings'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@shared/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@shared/ui/popover'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { BranchGroup } from '@features/git/branch-group'

type BranchSwitcherProps = {
    branches: GitBranchInfo[]
    currentBranch: string | null
    disabled: boolean
    onCheckout: (name: string) => void
    onCheckoutRemote: (remoteRef: string) => void
    onCreate: (name: string) => void
}

export const BranchSwitcher: FC<BranchSwitcherProps> = ({ branches, currentBranch, disabled, onCheckout, onCheckoutRemote, onCreate }) => {
    const { t } = useTranslation()
    const [open, setOpen] = useState(false)
    const [filter, setFilter] = useState('')

    const trimmedFilter = filter.trim()
    const hasExactMatch = branches.some((branch) => branch.name === trimmedFilter)
    const localBranches = branches.filter((branch) => !branch.isRemote)
    const remoteBranches = branches.filter((branch) => branch.isRemote)

    const handleSelect = (name: string) => {
        setOpen(false)
        setFilter('')
        const branch = branches.find((candidate) => candidate.name === name)
        if (branch?.isRemote) {
            onCheckoutRemote(name)
            return
        }
        onCheckout(name)
    }

    const handleCreate = () => {
        setOpen(false)
        setFilter('')
        onCreate(trimmedFilter)
    }

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <Tooltip>
                <TooltipTrigger asChild>
                    <PopoverTrigger
                        disabled={disabled}
                        aria-label={t('git.branchSwitch')}
                        className='hover:bg-explorer-item-hover flex min-w-0 items-center gap-1.5 rounded-sm px-1 py-0.5 disabled:opacity-50'>
                        <GitBranch className='size-3.5 shrink-0' />
                        <span className='truncate font-medium'>{currentBranch ?? t('git.noRepositoryLabel')}</span>
                    </PopoverTrigger>
                </TooltipTrigger>
                <TooltipContent side='bottom'>{t('git.branchSwitch')}</TooltipContent>
            </Tooltip>
            <PopoverContent align='start' className='w-64 p-0'>
                <Command shouldFilter={false} className='bg-panel-background text-app-foreground'>
                    <CommandInput value={filter} onValueChange={setFilter} placeholder={t('git.branchFilterPlaceholder')} />
                    <CommandList>
                        <CommandEmpty>{t('git.branchNotFound')}</CommandEmpty>
                        {trimmedFilter && !hasExactMatch && (
                            <CommandGroup>
                                <CommandItem onSelect={handleCreate}>
                                    <Plus className='size-4' />
                                    <span className='truncate'>{t('git.branchCreateNamed', { name: trimmedFilter })}</span>
                                </CommandItem>
                            </CommandGroup>
                        )}
                        <BranchGroup heading={t('git.branchLocal')} branches={localBranches} filter={trimmedFilter} onSelect={handleSelect} />
                        <BranchGroup heading={t('git.branchRemote')} branches={remoteBranches} filter={trimmedFilter} onSelect={handleSelect} />
                    </CommandList>
                </Command>
            </PopoverContent>
        </Popover>
    )
}
