import type { TicketSummary } from '../../ticket-loader'

export const MOCK_TICKETS = [
  {
    id: 't-001',
    title: 'Implement player movement',
    status: 'ToDo',
    priority: 'High',
    goal: 'gameplay',
    spec_ref: 'spec-01',
  },
  {
    id: 't-002',
    title: 'Add jump mechanic',
    status: 'InProgress',
    priority: 'Critical',
    goal: 'gameplay',
    spec_ref: 'spec-01',
    blockers: ['t-001'],
  },
  {
    id: 't-003',
    title: 'Fix collision bug',
    status: 'Done',
    priority: 'Medium',
    goal: 'bugfix',
    spec_ref: 'spec-02',
  },
  {
    id: 't-004',
    title: 'Blocked feature',
    status: 'Blocked',
    priority: 'Low',
    goal: 'feature',
    spec_ref: 'spec-03',
    blockers: ['t-003'],
  },
] as const

export const TICKET_SUMMARY_NO_DONE: TicketSummary = {
  tickets: MOCK_TICKETS.filter((ticket) => ticket.status !== 'Done'),
  byStatus: { ToDo: 2, InProgress: 1, Blocked: 1 },
  activeTickets: [MOCK_TICKETS[1]],
  incompleteCount: 3,
}

export const TICKET_SUMMARY_ALL_DONE: TicketSummary = {
  tickets: MOCK_TICKETS.map((ticket) => ({ ...ticket, status: 'Done' as const })),
  byStatus: { Done: 4 },
  activeTickets: [],
  incompleteCount: 0,
}
