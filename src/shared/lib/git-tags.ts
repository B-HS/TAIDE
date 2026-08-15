import type { TagInfo } from '@shared/api/bindings'

export const tagsTargetingCommit = (tags: TagInfo[], commitId: string) => tags.filter((tag) => tag.target === commitId)
