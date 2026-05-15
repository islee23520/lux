import React from 'react';
import type { Ticket } from '../../hooks/useKanban';

interface TicketDetailModalProps {
  ticket: Ticket | null;
  allTickets: Ticket[];
  onClose: () => void;
}

const formatDate = (value: string): string => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
};

export const TicketDetailModal: React.FC<TicketDetailModalProps> = ({ ticket, allTickets, onClose }) => {
  if (!ticket) {
    return null;
  }

  const blockers = ticket.blockers.map((blockerId) => {
    return allTickets.find((candidate) => candidate.id === blockerId) ?? blockerId;
  });
  const relatedSpecs = [ticket.spec_ref, ...ticket.tags.filter((tag) => tag.toLowerCase().startsWith('spec:'))]
    .filter((spec): spec is string => Boolean(spec));

  return (
    <div className="kanban-modal" role="dialog" aria-modal="true" aria-labelledby="kanban-modal-title">
      <div className="kanban-modal__backdrop" onClick={onClose} />
      <section className="kanban-modal__panel" aria-labelledby="kanban-modal-title">
        <header className="kanban-modal__header">
          <div>
            <p className="kanban-modal__eyebrow">{ticket.priority} · {ticket.status}</p>
            <h2 id="kanban-modal-title">{ticket.title}</h2>
          </div>
          <button type="button" onClick={onClose} aria-label="Close ticket detail">×</button>
        </header>

        <div className="kanban-modal__body">
          <section>
            <h3>Description</h3>
            <p>{ticket.description || 'No description provided.'}</p>
          </section>

          <section>
            <h3>Blockers</h3>
            {blockers.length > 0 ? (
              <ul className="kanban-modal__list">
                {blockers.map((blocker) => (
                  <li key={typeof blocker === 'string' ? blocker : blocker.id}>
                    {typeof blocker === 'string' ? blocker : `${blocker.title} (${blocker.status})`}
                  </li>
                ))}
              </ul>
            ) : (
              <p>No blockers.</p>
            )}
          </section>

          <section>
            <h3>Related specs</h3>
            {relatedSpecs.length > 0 ? (
              <div className="kanban-modal__chips">
                {relatedSpecs.map((spec) => <span key={spec}>{spec}</span>)}
              </div>
            ) : (
              <p>No spec reference.</p>
            )}
          </section>

          <section>
            <h3>Progress log</h3>
            <ol className="kanban-modal__timeline">
              <li><span>Created</span><time dateTime={ticket.created_at}>{formatDate(ticket.created_at)}</time></li>
              <li><span>Last updated</span><time dateTime={ticket.updated_at}>{formatDate(ticket.updated_at)}</time></li>
            </ol>
          </section>
        </div>
      </section>
    </div>
  );
};
