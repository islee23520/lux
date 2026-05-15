import React from 'react';
import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import type { Ticket, TicketStatus } from '../../hooks/useKanban';
import { TicketCard } from './TicketCard';

interface KanbanColumnProps {
  status: TicketStatus;
  title: string;
  tickets: Ticket[];
  onOpenTicket: (ticket: Ticket) => void;
}

export const KanbanColumn: React.FC<KanbanColumnProps> = ({ status, title, tickets, onOpenTicket }) => {
  const { setNodeRef, isOver } = useDroppable({ id: status, data: { type: 'column', status } });

  return (
    <section
      ref={setNodeRef}
      className="kanban-column"
      style={{ outline: isOver ? '1px solid var(--blue, #38bdf8)' : '1px solid transparent' }}
      aria-label={`${title} column`}
    >
      <header className="kanban-column__header">
        <h3>{title}</h3>
        <span>{tickets.length}</span>
      </header>
      <SortableContext items={tickets.map((ticket) => ticket.id)} strategy={verticalListSortingStrategy}>
        <div className="kanban-column__tickets">
          {tickets.map((ticket) => (
            <TicketCard key={ticket.id} ticket={ticket} onOpen={onOpenTicket} />
          ))}
          {tickets.length === 0 && <p className="kanban-column__empty">No tickets</p>}
        </div>
      </SortableContext>
    </section>
  );
};
