export const ACTIVE_STATE = {
  status: 'Active' as const,
  continuation_count: 5,
  consecutive_failures: 0,
  current_ticket_id: 't-002',
  session_id: 'sess-abc',
  started_at: '2026-05-13T10:00:00Z',
  last_ambiguity: '0.05',
  stagnation_count: 0,
}

export const STOPPED_STATE = {
  ...ACTIVE_STATE,
  status: 'Stopped' as const,
    stop_reason: 'max_continuations_reached' as const,
  continuation_count: 50,
}

export const ERROR_STATE = {
  ...ACTIVE_STATE,
  status: 'Error' as const,
  consecutive_failures: 3,
}
