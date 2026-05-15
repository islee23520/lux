import React from 'react';
import { useSortable } from '@dnd-kit/sortable';
import type { Ticket } from '../../hooks/useKanban';

interface TicketCardProps {
  ticket: Ticket;
  onOpen: (ticket: Ticket) => void;
}

const priorityColors: Record<Ticket['priority'], string> = {
  Critical: '#f87171',
  High: '#fb923c',
  Medium: '#facc15',
  Low: '#38bdf8',
};

const formatStatus = (status: Ticket['status']): string => {
  return status.replace(/([a-z])([A-Z])/g, '$1 $2');
};

export const TicketCard: React.FC<TicketCardProps> = ({ ticket, onOpen }) => {
  const hasBlockers = ticket.status === 'Blocked' || ticket.blockers.length > 0;
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: ticket.id,
    data: { type: 'ticket', ticketId: ticket.id, status: ticket.status },
  });

  const transformStyle = transform
    ? `translate3d(${Math.round(transform.x)}px, ${Math.round(transform.y)}px, 0)`
    : undefined;

  return (
    <article
      ref={setNodeRef}
      className="kanban-ticket-card"
      style={{
        transform: transformStyle,
        transition,
        opacity: isDragging ? 0.62 : 1,
        borderColor: hasBlockers ? 'rgba(248, 113, 113, 0.86)' : 'var(--line, rgba(148, 163, 184, 0.2))',
        boxShadow: hasBlockers ? '0 0 0 1px rgba(248, 113, 113, 0.16)' : 'none',
      }}
      {...attributes}
    >
      <div className="kanban-ticket-card__topline">
        <span
          className="kanban-ticket-card__priority"
          style={{ color: priorityColors[ticket.priority], borderColor: `${priorityColors[ticket.priority]}66` }}
        >
          {ticket.priority}
        </span>
        {hasBlockers && <span className="kanban-ticket-card__blocker" aria-label="Has blockers">● Blocked</span>}
      </div>

      <button type="button" className="kanban-ticket-card__title" onClick={() => onOpen(ticket)}>
        {ticket.title}
      </button>

      <div className="kanban-ticket-card__meta">
        <span>{formatStatus(ticket.status)}</span>
        {ticket.spec_ref && <span className="kanban-ticket-card__spec">Spec: {ticket.spec_ref}</span>}
      </div>

      {ticket.tags.length > 0 && (
        <div className="kanban-ticket-card__tags" aria-label="Ticket tags">
          {ticket.tags.slice(0, 3).map((tag) => (
            <span key={tag}>{tag}</span>
          ))}
        </div>
      )}

      <button type="button" className="kanban-ticket-card__drag" {...listeners} aria-label={`Drag ${ticket.title}`}>
        Drag
      </button>
    </article>
  );
};
