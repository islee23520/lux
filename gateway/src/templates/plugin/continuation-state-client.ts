export type ContinuationStatus = "Idle" | "Active" | "Stopped" | "Error" | "Complete"

export interface ContinuationState {
  session_id: string | null
  continuation_count: number
  stagnation_count: number
  consecutive_failures: number
  last_ambiguity: string | null
  last_ticket_baseline: string | null
  current_ticket_id: string | null
  status: ContinuationStatus
  started_at: string | null
  updated_at: string
  stop_reason: string | null
}

export interface ContinuationStateWriteOptions {
  gatewayUrl: string
  projectPath: string
  expectedSeq: number
  expectedStatus?: string
}

export async function readContinuationState(opts: { gatewayUrl: string; projectPath: string }): Promise<ContinuationState> {
  const url = new URL(
    `/api/lux/continuation/state?project_path=${encodeURIComponent(opts.projectPath)}`,
    opts.gatewayUrl,
  )

  const response = await fetch(url.toString(), {
    method: "GET",
    headers: { "Accept": "application/json" },
  })
  if (!response.ok) {
    const text = await response.text().catch(() => "")
    throw new Error(`readContinuationState failed (HTTP ${response.status}): ${text}`)
  }
  return await response.json() as ContinuationState
}

export interface ContinuationWriteResult {
  seq: number
}

export async function writeContinuationState(
  opts: ContinuationStateWriteOptions,
  state: ContinuationState,
): Promise<ContinuationWriteResult> {
  const url = new URL(
    `/api/lux/continuation/state?project_path=${encodeURIComponent(opts.projectPath)}`,
    opts.gatewayUrl,
  )

  const payload = {
    expected_seq: opts.expectedSeq,
    expected_status: opts.expectedStatus,
    session_id: state.session_id,
    continuation_count: state.continuation_count,
    stagnation_count: state.stagnation_count,
    consecutive_failures: state.consecutive_failures,
    last_ambiguity: state.last_ambiguity,
    last_ticket_baseline: state.last_ticket_baseline,
    current_ticket_id: state.current_ticket_id,
    status: state.status,
    started_at: state.started_at,
    stop_reason: state.stop_reason,
  }

  const response = await fetch(url.toString(), {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  })

  if (!response.ok) {
    const text = await response.text().catch(() => "")
    throw new Error(`continuation state write failed (HTTP ${response.status}): ${text}`)
  }

  const body = await response.json() as { seq?: number }
  return { seq: body.seq ?? opts.expectedSeq + 1 }
}

export async function updateContinuationState(
  opts: ContinuationStateWriteOptions,
  partial: Partial<ContinuationState>,
): Promise<ContinuationState> {
  const current = await readContinuationState({ gatewayUrl: opts.gatewayUrl, projectPath: opts.projectPath })
  const merged: ContinuationState = { ...current, ...partial }
  await writeContinuationState(opts, merged)
  return merged
}
