import type { ProjectGroup, ProjectGroupId, ProjectId, ProjectRef } from '@shared/api/bindings'
import { PROJECT_COLOR_VAR_PREFIX } from '@shared/constants/project-display'
import { PROJECT_GROUP_NAME_MAX_CODEPOINTS } from '@shared/constants/project-group'
import { resolveProjectColorToken } from '@shared/lib/project-display'

const CONTROL_CHARACTER_PATTERN = /\p{Cc}/gu

const toGroupNameText = (value: string) => [...value.replace(CONTROL_CHARACTER_PATTERN, '')].slice(0, PROJECT_GROUP_NAME_MAX_CODEPOINTS).join('')

/**
 * What the name input may hold while typing: control characters dropped and the length capped in
 * codepoints. Deliberately does *not* trim, so a space typed between two words survives the
 * keystroke — the same split {@link normalizeProjectGroupName} completes at submit time
 * (`project-display.ts`'s `clampProjectLabel`/`normalizeProjectLabel` pair).
 */
export const clampProjectGroupName = (value: string) => toGroupNameText(value)

/** What is actually sent, in the order `domain::project::service::sanitize_group_name` applies it: strip control characters, trim, then measure. */
export const normalizeProjectGroupName = (value: string) => toGroupNameText(value.replace(CONTROL_CHARACTER_PATTERN, '').trim())

/** The group header's tint, or `null` when nothing valid is stored — the palette is `ProjectDisplay`'s own (`resolveProjectColorToken`), which is also the allow-list Rust validates against. */
export const resolveProjectGroupColorVar = (color: string | null | undefined) => {
    const token = resolveProjectColorToken(color)
    return token === null ? null : `var(${PROJECT_COLOR_VAR_PREFIX}${token})`
}

export type ProjectGroupSection = { group: ProjectGroup; members: ProjectRef[] }

/**
 * How the rail is laid out: one section per group in the groups' own order, then everything the
 * groups do not claim.
 *
 * Members are taken by *filtering the open project list*, never by mapping over `group.members` —
 * `session.projects` is the single truth for both "is it open" and "in what order"
 * (contract §1.A), and a group's member list is a membership set that may well name projects that
 * are merely known to this machine. Mapping the other way round would draw closed projects in the
 * rail and let a group's internal order disagree with the order a drag just wrote.
 *
 * A project the server somehow left in two groups lands in the last of them rather than both, so the
 * sections stay a partition of the rail no matter what the session holds.
 */
export const resolveProjectGroupSections = (projects: ProjectRef[], groups: ProjectGroup[]) => {
    const groupIdByProjectId = new Map<ProjectId, ProjectGroupId>()
    for (const group of groups) for (const memberId of group.members ?? []) groupIdByProjectId.set(memberId, group.id)

    const sections: ProjectGroupSection[] = groups.map((group) => ({
        group,
        members: projects.filter((project) => groupIdByProjectId.get(project.id) === group.id),
    }))

    return { sections, ungrouped: projects.filter((project) => !groupIdByProjectId.has(project.id)), groupIdByProjectId }
}

/**
 * The membership list "그룹에 추가" sends. Appending is enough to *move* a project between groups:
 * `project_group_set_members` takes it away from whatever group held it before, since a project
 * belongs to at most one group (`types::ProjectGroup`).
 */
export const withProjectGroupMember = (members: ProjectId[] | undefined, projectId: ProjectId) =>
    members?.includes(projectId) ? members : [...(members ?? []), projectId]

export const withoutProjectGroupMember = (members: ProjectId[] | undefined, projectId: ProjectId) =>
    (members ?? []).filter((memberId) => memberId !== projectId)
