import React, { useMemo, useState } from 'react';
import { DndContext, PointerSensor, useSensor, useSensors } from '@dnd-kit/core';
import type { DragEndEvent } from '@dnd-kit/core';
import { KanbanColumn } from './KanbanColumn';
import { TicketDetailModal } from './TicketDetailModal';
import { useKanban } from '../../hooks/useKanban';
import type { Ticket, TicketStatus } from '../../hooks/useKanban';

interface KanbanBoardProps {
  projectPath: string;
}

const columns: Array<{ status: TicketStatus; title: string }> = [
  { status: 'Backlog', title: 'Backlog' },
  { status: 'Blocked', title: 'Blocked' },
  { status: 'ToDo', title: 'To Do' },
  { status: 'InProgress', title: 'In Progress' },
  { status: 'Done', title: 'Done' },
];

const kanbanStyles = `
.kanban-board { min-height: 100%; color: var(--text, #f5f7fb); background: var(--bg, #0b0f17); padding: 20px; box-sizing: border-box; }
.kanban-board__header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 18px; }
.kanban-board__header h2 { margin: 0; font-size: 1.35rem; color: var(--text, #f5f7fb); }
.kanban-board__header p { margin: 4px 0 0; color: var(--muted, #9aa7bc); }
.kanban-board__actions { display: flex; align-items: center; gap: 10px; }
.kanban-board__button { border: 1px solid var(--line, rgba(148, 163, 184, 0.2)); background: rgba(56, 189, 248, 0.1); color: var(--blue, #38bdf8); border-radius: 8px; padding: 8px 12px; cursor: pointer; }
.kanban-board__button:disabled { cursor: not-allowed; opacity: 0.6; }
.kanban-board__error { color: #f87171; font-size: 0.85rem; }
.kanban-board__columns { display: grid; grid-template-columns: repeat(5, minmax(220px, 1fr)); gap: 12px; overflow-x: auto; padding-bottom: 8px; }
.kanban-column { min-height: 520px; border: 1px solid var(--line, rgba(148, 163, 184, 0.2)); border-radius: 14px; background: rgba(18, 26, 42, 0.74); padding: 12px; transition: outline-color 0.16s ease, background 0.16s ease; }
.kanban-column__header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
.kanban-column__header h3 { margin: 0; font-size: 0.92rem; letter-spacing: 0.02em; }
.kanban-column__header span { min-width: 24px; height: 24px; border-radius: 999px; display: grid; place-items: center; background: rgba(148, 163, 184, 0.12); color: var(--muted, #9aa7bc); font-size: 0.78rem; }
.kanban-column__tickets { display: flex; flex-direction: column; gap: 10px; min-height: 456px; }
.kanban-column__empty { margin: 8px 0; padding: 18px; border: 1px dashed var(--line, rgba(148, 163, 184, 0.2)); border-radius: 12px; color: var(--muted, #9aa7bc); text-align: center; }
.kanban-ticket-card { position: relative; border: 1px solid var(--line, rgba(148, 163, 184, 0.2)); border-radius: 12px; background: var(--panel-strong, rgba(18, 26, 42, 0.94)); padding: 12px; cursor: default; }
.kanban-ticket-card__topline, .kanban-ticket-card__meta, .kanban-ticket-card__tags { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.kanban-ticket-card__priority, .kanban-ticket-card__tags span, .kanban-modal__chips span { border: 1px solid; border-radius: 999px; padding: 2px 7px; font-size: 0.72rem; background: rgba(148, 163, 184, 0.08); }
.kanban-ticket-card__blocker { color: #f87171; font-size: 0.74rem; }
.kanban-ticket-card__title { width: 100%; margin: 10px 0 8px; padding: 0; border: 0; background: transparent; color: var(--text, #f5f7fb); text-align: left; font: inherit; font-weight: 700; cursor: pointer; }
.kanban-ticket-card__title:hover { color: var(--blue, #38bdf8); }
.kanban-ticket-card__meta { color: var(--muted, #9aa7bc); font-size: 0.76rem; justify-content: space-between; }
.kanban-ticket-card__spec { color: var(--blue, #38bdf8); }
.kanban-ticket-card__tags { margin-top: 10px; color: var(--muted, #9aa7bc); }
.kanban-ticket-card__tags span { border-color: rgba(148, 163, 184, 0.22); }
.kanban-ticket-card__drag { margin-top: 10px; width: 100%; border: 1px solid rgba(148, 163, 184, 0.18); background: rgba(148, 163, 184, 0.08); color: var(--muted, #9aa7bc); border-radius: 8px; padding: 6px; cursor: grab; }
.kanban-modal { position: fixed; inset: 0; z-index: 60; display: grid; place-items: center; padding: 24px; }
.kanban-modal__backdrop { position: absolute; inset: 0; background: rgba(3, 7, 18, 0.74); }
.kanban-modal__panel { position: relative; width: min(720px, 100%); max-height: min(760px, 88vh); overflow: auto; border: 1px solid var(--line, rgba(148, 163, 184, 0.2)); border-radius: 16px; background: var(--panel-strong, rgba(18, 26, 42, 0.94)); box-shadow: 0 24px 80px rgba(0, 0, 0, 0.45); }
.kanban-modal__header { display: flex; justify-content: space-between; gap: 16px; padding: 18px 20px; border-bottom: 1px solid var(--line, rgba(148, 163, 184, 0.2)); }
.kanban-modal__header h2 { margin: 4px 0 0; }
.kanban-modal__header button { border: 0; background: transparent; color: var(--text, #f5f7fb); font-size: 1.5rem; cursor: pointer; }
.kanban-modal__eyebrow { margin: 0; color: var(--blue, #38bdf8); font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.08em; }
.kanban-modal__body { display: grid; gap: 18px; padding: 20px; }
.kanban-modal__body h3 { margin: 0 0 8px; font-size: 0.86rem; color: var(--muted, #9aa7bc); text-transform: uppercase; letter-spacing: 0.08em; }
.kanban-modal__body p { margin: 0; line-height: 1.55; }
.kanban-modal__list { margin: 0; padding-left: 20px; }
.kanban-modal__chips { display: flex; gap: 8px; flex-wrap: wrap; }
.kanban-modal__chips span { border-color: rgba(56, 189, 248, 0.42); color: var(--blue, #38bdf8); }
.kanban-modal__timeline { display: grid; gap: 8px; margin: 0; padding-left: 20px; }
.kanban-modal__timeline li { color: var(--muted, #9aa7bc); }
.kanban-modal__timeline span { color: var(--text, #f5f7fb); margin-right: 8px; }
@media (max-width: 900px) { .kanban-board__columns { grid-template-columns: repeat(5, minmax(240px, 82vw)); } .kanban-board__header { flex-direction: column; } }
`;

export const KanbanBoard: React.FC<KanbanBoardProps> = ({ projectPath }) => {
  const { tickets, ticketsByStatus, loading, error, refreshTickets, updateTicketStatus } = useKanban(projectPath);
  const [selectedTicket, setSelectedTicket] = useState<Ticket | null>(null);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));

  const ticketsById = useMemo(() => {
    return tickets.reduce<Record<string, Ticket>>((acc, ticket) => {
      acc[ticket.id] = ticket;
      return acc;
    }, {});
  }, [tickets]);

  const resolveTargetStatus = (id: string): TicketStatus | null => {
    const column = columns.find((candidate) => candidate.status === id);
    if (column) {
      return column.status;
    }
    return ticketsById[id]?.status ?? null;
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const ticket = ticketsById[String(event.active.id)];
    const overId = event.over ? String(event.over.id) : null;
    if (!ticket || !overId) {
      return;
    }

    const nextStatus = resolveTargetStatus(overId);
    if (!nextStatus || nextStatus === ticket.status) {
      return;
    }

    void updateTicketStatus(ticket, nextStatus).catch(() => {
      void refreshTickets();
    });
  };

  return (
    <main className="kanban-board">
      <style>{kanbanStyles}</style>
      <header className="kanban-board__header">
        <div>
          <h2>Kanban Board</h2>
          <p>Backlog → Blocked → To Do → In Progress → Done</p>
        </div>
        <div className="kanban-board__actions">
          {error && <span className="kanban-board__error">{error}</span>}
          <button type="button" className="kanban-board__button" onClick={() => void refreshTickets()} disabled={loading}>
            {loading ? 'Refreshing…' : 'Refresh'}
          </button>
        </div>
      </header>

      <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
        <div className="kanban-board__columns">
          {columns.map((column) => (
            <KanbanColumn
              key={column.status}
              status={column.status}
              title={column.title}
              tickets={ticketsByStatus[column.status]}
              onOpenTicket={setSelectedTicket}
            />
          ))}
        </div>
      </DndContext>

      <TicketDetailModal ticket={selectedTicket} allTickets={tickets} onClose={() => setSelectedTicket(null)} />
    </main>
  );
};
