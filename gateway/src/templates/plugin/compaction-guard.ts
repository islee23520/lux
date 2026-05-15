import type { LuxSessionState } from "./session-state"

export const COMPACTION_GUARD_MS = 60_000

export function armCompactionGuard(state: LuxSessionState, epoch = Date.now()): void {
  state.recentCompactionAt = epoch
  state.recentCompactionEpoch += 1
  state.acknowledgedCompactionEpoch = 0
}

export function isCompactionGuardActive(
  state: LuxSessionState,
  currentEpoch = Date.now(),
  guardMs = COMPACTION_GUARD_MS,
): boolean {
  if (state.recentCompactionAt === null) return false
  if (state.acknowledgedCompactionEpoch >= state.recentCompactionEpoch) return false
  return currentEpoch - state.recentCompactionAt < guardMs
}

export function acknowledgeCompaction(state: LuxSessionState, epoch = state.recentCompactionEpoch): void {
  state.acknowledgedCompactionEpoch = epoch
}
