import { useCallback, useEffect, useMemo, useState } from 'react';

export type TicketStatus = 'Backlog' | 'Blocked' | 'ToDo' | 'InProgress' | 'Done';

export type TicketPriority = 'Critical' | 'High' | 'Medium' | 'Low';

export interface Ticket {
  id: string;
  title: string;
  description: string;
  status: TicketStatus;
  priority: TicketPriority;
  assignee: string | null;
  blockers: string[];
  tags: string[];
  spec_ref: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateTicketInput {
  title: string;
  description: string;
  priority: TicketPriority;
  tags?: string[];
  spec_ref?: string | null;
}

export interface UseKanbanResult {
  tickets: Ticket[];
  ticketsByStatus: Record<TicketStatus, Ticket[]>;
  loading: boolean;
  error: string | null;
  refreshTickets: () => Promise<void>;
  createTicket: (input: CreateTicketInput) => Promise<Ticket>;
  updateTicket: (ticket: Ticket) => Promise<Ticket>;
  updateTicketStatus: (ticket: Ticket, status: TicketStatus) => Promise<Ticket>;
  deleteTicket: (id: string) => Promise<void>;
}

const TICKET_STATUSES: TicketStatus[] = ['Backlog', 'Blocked', 'ToDo', 'InProgress', 'Done'];

const isRecord = (value: unknown): value is Record<string, unknown> => {
  return typeof value === 'object' && value !== null;
};

const isKanbanUpdateMessage = (value: unknown): boolean => {
  if (!isRecord(value)) {
    return false;
  }

  if (value.type === 'kanban:update' || value.kind === 'kanban:update') {
    return true;
  }

  const event = value.event;
  if (isRecord(event) && (event.type === 'kanban:update' || event.kind === 'kanban:update')) {
    return true;
  }

  const payload = value.payload;
  return isRecord(payload) && (payload.type === 'kanban:update' || payload.kind === 'kanban:update');
};

const buildProjectQuery = (projectPath: string): string => {
  return new URLSearchParams({ project_path: projectPath }).toString();
};

const parseApiError = async (response: Response): Promise<string> => {
  try {
    const body = (await response.json()) as unknown;
    if (isRecord(body) && typeof body.error === 'string') {
      return body.error;
    }
  } catch {
    return response.statusText;
  }
  return response.statusText;
};

const fetchJson = async <T>(input: RequestInfo | URL, init?: RequestInit): Promise<T> => {
  const response = await fetch(input, init);
  if (!response.ok) {
    throw new Error(await parseApiError(response));
  }
  return (await response.json()) as T;
};

export function useKanban(projectPath: string): UseKanbanResult {
  const [tickets, setTickets] = useState<Ticket[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshTickets = useCallback(async () => {
    if (!projectPath) {
      setTickets([]);
      setError('Project path is required to load kanban tickets.');
      return;
    }

    setLoading(true);
    try {
      const query = buildProjectQuery(projectPath);
      const nextTickets = await fetchJson<Ticket[]>(`/api/lux/kanban/tickets?${query}`);
      setTickets(nextTickets);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load kanban tickets.');
    } finally {
      setLoading(false);
    }
  }, [projectPath]);

  const createTicket = useCallback(async (input: CreateTicketInput) => {
    const created = await fetchJson<Ticket>('/api/lux/kanban/tickets', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        project_path: projectPath,
        title: input.title,
        description: input.description,
        priority: input.priority,
        tags: input.tags ?? [],
        spec_ref: input.spec_ref ?? null,
      }),
    });
    setTickets((current) => [created, ...current]);
    return created;
  }, [projectPath]);

  const updateTicket = useCallback(async (ticket: Ticket) => {
    const updated = await fetchJson<Ticket>(`/api/lux/kanban/tickets/${encodeURIComponent(ticket.id)}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ project_path: projectPath, ticket }),
    });
    setTickets((current) => current.map((item) => (item.id === updated.id ? updated : item)));
    return updated;
  }, [projectPath]);

  const updateTicketStatus = useCallback(async (ticket: Ticket, status: TicketStatus) => {
    const nextTicket: Ticket = { ...ticket, status };
    return updateTicket(nextTicket);
  }, [updateTicket]);

  const deleteTicket = useCallback(async (id: string) => {
    const response = await fetch(`/api/lux/kanban/tickets/${encodeURIComponent(id)}`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ project_path: projectPath }),
    });
    if (!response.ok) {
      throw new Error(await parseApiError(response));
    }
    setTickets((current) => current.filter((ticket) => ticket.id !== id));
  }, [projectPath]);

  useEffect(() => {
    void refreshTickets();
  }, [refreshTickets]);

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const socket = new WebSocket(`${protocol}//${window.location.host}/events?role=kanban-ui`);

    socket.addEventListener('message', (message) => {
      try {
        const parsed = JSON.parse(String(message.data)) as unknown;
        if (isKanbanUpdateMessage(parsed)) {
          void refreshTickets();
        }
      } catch {
        // Ignore non-JSON event frames.
      }
    });

    socket.addEventListener('error', () => {
      socket.close();
    });

    return () => socket.close();
  }, [refreshTickets]);

  const ticketsByStatus = useMemo(() => {
    const grouped = TICKET_STATUSES.reduce<Record<TicketStatus, Ticket[]>>((acc, status) => {
      acc[status] = [];
      return acc;
    }, {
      Backlog: [],
      Blocked: [],
      ToDo: [],
      InProgress: [],
      Done: [],
    });

    for (const ticket of tickets) {
      grouped[ticket.status].push(ticket);
    }

    return grouped;
  }, [tickets]);

  return {
    tickets,
    ticketsByStatus,
    loading,
    error,
    refreshTickets,
    createTicket,
    updateTicket,
    updateTicketStatus,
    deleteTicket,
  };
}
