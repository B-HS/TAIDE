import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Terminal } from 'lucide-react'
import type { Task, TaskSource } from '@shared/api/bindings'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import { requestEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { subscribeOpenTaskRunner } from '@shared/lib/task-runner-bridge'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@shared/ui/command'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { activeProjectQueryOptions } from '@entities/project/project.query'
import { tasksQueryOptions } from '@entities/task/task.query'

const TASK_SOURCE_LABEL_KEY: Record<TaskSource, string> = {
    npm: 'task.sourceNpm',
    make: 'task.sourceMake',
    cargo: 'task.sourceCargo',
}

export const TaskRunnerDialog = () => {
    const [open, setOpen] = useState(false)
    const [query, setQuery] = useState('')

    const { t } = useTranslation()
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: tasks, isPending } = useQuery({ ...tasksQueryOptions(activeProjectId), enabled: open && !!activeProjectId })

    const handleOpenChange = (next: boolean) => {
        setOpen(next)
        if (!next) setQuery('')
    }

    const filteredTasks = fuzzyFilter(query, tasks ?? [], (task) => task.label)

    const runTask = (task: Task) => {
        requestEditorPaneCommand({ type: 'run-in-terminal', text: task.command, cwd: task.cwd })
        handleOpenChange(false)
    }

    useEffect(() => subscribeOpenTaskRunner(() => setOpen(true)), [])

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogHeader className='sr-only'>
                <DialogTitle>{t('task.runTask')}</DialogTitle>
            </DialogHeader>
            <DialogContent className='overflow-hidden p-0' showCloseButton={false}>
                <Command shouldFilter={false} className='bg-panel-background text-app-foreground'>
                    <CommandInput value={query} onValueChange={setQuery} placeholder={t('task.runTask')} />
                    <CommandList>
                        <CommandEmpty>{isPending ? t('common.loading') : t('task.noTasksFound')}</CommandEmpty>
                        <CommandGroup>
                            {filteredTasks.map(({ item }) => (
                                <CommandItem key={`${item.source}:${item.cwd}:${item.label}`} onSelect={() => runTask(item)}>
                                    <Terminal className='size-4' />
                                    <span className='truncate'>{item.label}</span>
                                    <span className='truncate text-xs text-muted-foreground'>{t(TASK_SOURCE_LABEL_KEY[item.source])}</span>
                                </CommandItem>
                            ))}
                        </CommandGroup>
                    </CommandList>
                </Command>
            </DialogContent>
        </Dialog>
    )
}
