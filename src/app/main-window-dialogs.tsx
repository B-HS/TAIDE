import { useFocusedProjectId } from '@app/providers/shell-slot-provider'
import { CommandPalette } from '@widgets/command-palette/command-palette'
import { TaskRunnerDialog } from '@widgets/task-runner/task-runner-dialog'

/**
 * The main window's half of the palette/task-runner rescoping (d-62 §1.D): both dialogs take the
 * project they act on as a prop, so somebody inside the provider tree has to resolve it for them.
 * `App` itself cannot — it *renders* `AppProviders`, so a hook in its body would run outside the
 * `QueryClientProvider` — and `AppShell` is not it either, since these two are deliberately siblings
 * of the shell rather than children of it (a dialog must not unmount when the shell swaps its body
 * for the Welcome screen).
 *
 * That project is the *focused shell slot's* (d-62 §1.B), not the window's: with two projects open
 * side by side, ⌘P has to search the one the user is looking at. `useFocusedProjectId` falls back to
 * the global active-project session for the first paint before the slot tree loads, and those two
 * agree by definition once it has.
 *
 * The auxiliary branch needs no equivalent: its project is a constant carried in the window's own
 * URL, so `app.tsx` passes it straight through.
 */
export const MainWindowDialogs = () => {
    const projectId = useFocusedProjectId()

    return (
        <>
            <CommandPalette projectId={projectId} />
            <TaskRunnerDialog projectId={projectId} />
        </>
    )
}
