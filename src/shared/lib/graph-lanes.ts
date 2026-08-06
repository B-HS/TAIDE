export type GraphCommitInput = {
    id: string
    parents: string[]
}

export type GraphEdge = {
    toLane: number
    parentId: string
}

export type GraphNode = {
    id: string
    lane: number
    color: number
    continuesFromAbove: boolean
    edges: GraphEdge[]
    passthroughLanes: number[]
}

const LANE_COLOR_COUNT = 12

const findOpenLane = (openLanes: (string | null)[], id: string) => openLanes.indexOf(id)

const claimFreeLane = (openLanes: (string | null)[]) => {
    const freeIndex = openLanes.indexOf(null)
    if (freeIndex !== -1) return freeIndex
    openLanes.push(null)
    return openLanes.length - 1
}

const computeNode = (commit: GraphCommitInput, openLanes: (string | null)[]) => {
    const openLanesBefore = [...openLanes]

    const foundLane = findOpenLane(openLanes, commit.id)
    const lane = foundLane === -1 ? claimFreeLane(openLanes) : foundLane
    const continuesFromAbove = foundLane !== -1

    const [firstParentId, ...restParentIds] = commit.parents
    const edges: GraphEdge[] = []

    if (firstParentId) {
        const existingLane = findOpenLane(openLanes, firstParentId)
        if (existingLane !== -1 && existingLane !== lane) {
            openLanes[lane] = null
            edges.push({ toLane: existingLane, parentId: firstParentId })
        } else {
            openLanes[lane] = firstParentId
            edges.push({ toLane: lane, parentId: firstParentId })
        }
    } else {
        openLanes[lane] = null
    }

    for (const parentId of restParentIds) {
        const existingLane = findOpenLane(openLanes, parentId)
        const parentLane = existingLane === -1 ? claimFreeLane(openLanes) : existingLane
        if (existingLane === -1) openLanes[parentLane] = parentId
        edges.push({ toLane: parentLane, parentId })
    }

    const passthroughLanes = openLanesBefore.reduce<number[]>((lanes, value, index) => {
        if (value !== null && index !== lane) lanes.push(index)
        return lanes
    }, [])

    return { id: commit.id, lane, color: lane % LANE_COLOR_COUNT, continuesFromAbove, edges, passthroughLanes }
}

export const computeGraphLanes = (commits: GraphCommitInput[]) => {
    const openLanes: (string | null)[] = []
    return commits.map((commit) => computeNode(commit, openLanes))
}
