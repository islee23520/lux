import { NextActionResult } from "./next-action-generator";
import { Ticket, TicketSummary } from "./ticket-loader";

export interface PromptBuilderContext {
  ticket: Ticket;
  nextAction: NextActionResult;
  ambiguity: number;
  summary: TicketSummary;
  continuationCount: number;
  consecutiveFailures: number;
  lastError?: string;
  aiContext?: AiContextSummary;
}

export interface AiContextSummary {
  ontology: {
    schemaVersion: string;
    requiredTerms: string[];
  };
  astSummary: {
    source: string;
    nodeCount?: number;
    nodeTypes: string[];
  };
  coordinateMappingSummary: {
    frames: string[];
    origins: string[];
  };
  evidenceGateRequirements: {
    requiredEvidence: string[];
    requiredReferences: string[];
  };
  blockers: Array<{
    kind: string;
    reason: string;
  }>;
}

function formatAcceptanceCriteria(ticket: Ticket): string {
  const raw = ticket.acceptance_criteria;
  const values = Array.isArray(raw)
    ? raw.filter((item): item is string => typeof item === "string" && item.length > 0)
    : typeof raw === "string" && raw.length > 0
      ? raw.split("\n").filter((item) => item.trim().length > 0)
      : [];

  if (values.length === 0) {
    return "- [ ] Complete the ticket according to its spec reference.";
  }

  return values.map((item) => `- [ ] ${item.replace(/^[-*]\s*\[\s*[ xX]?\s*\]\s*/, "").replace(/^[-*]\s+/, "")}`).join("\n");
}

export function buildContinuationPrompt(ctx: PromptBuilderContext): string {
  const { ticket, nextAction, ambiguity, summary, continuationCount, consecutiveFailures, lastError, aiContext } = ctx;

  const done = summary.tickets.filter((item) => item.status === "Done").length;
  const total = summary.tickets.length;

  const executionHint = consecutiveFailures > 0
    ? `Previous failures detected: ${consecutiveFailures}.${lastError ? ` Last error: ${lastError}.` : ""} Prioritize fixing the failing path before broadening scope.`
    : "";

  const aiContextSections = aiContext
    ? [
        "AI context:",
        `Ontology: schema=${aiContext.ontology.schemaVersion}; required=${aiContext.ontology.requiredTerms.join(", ")}`,
        `AST summary: source=${aiContext.astSummary.source}; nodes=${aiContext.astSummary.nodeCount ?? "unknown"}; types=${aiContext.astSummary.nodeTypes.join(", ") || "none"}`,
        `Coordinate mapping: frames=${aiContext.coordinateMappingSummary.frames.join(", ")}; origins=${aiContext.coordinateMappingSummary.origins.join(", ") || "none"}`,
        `Evidence gates: evidence=${aiContext.evidenceGateRequirements.requiredEvidence.join(", ")}; references=${aiContext.evidenceGateRequirements.requiredReferences.join(", ")}`,
        aiContext.blockers.length > 0
          ? `Blockers: ${aiContext.blockers.map((blocker) => `${blocker.kind} (${blocker.reason})`).join("; ")}`
          : "Blockers: none",
        "Constraints: pixel-only completion is forbidden. Fake engine parity is forbidden.",
        "",
      ]
    : [];

  const prompt = [
    `[Lux] Continue spec-driven implementation. ${nextAction.message}`,
    "",
    `Current focus: [${ticket.id ?? "unknown"}] ${ticket.title ?? "Untitled ticket"} (${ticket.spec_ref ?? "no spec_ref"})`,
    "Acceptance criteria:",
    formatAcceptanceCriteria(ticket),
    "",
    `Progress: ${done}/${total} tickets complete. Continuation #${continuationCount + 1}.`,
    executionHint,
    `Spec ambiguity: ${Math.round(ambiguity * 100)}%. Decision confidence: ${nextAction.confidence}.`,
    "",
    ...aiContextSections,
    "Preserve .lux as the source of truth. Update spec/tickets as work progresses.",
    "Do not ask for permission. Execute the next logical step.",
  ].filter((line) => line.length > 0).join("\n");

  if (prompt.length > 2000) {
    return prompt.substring(0, 1997) + "...";
  }

  return prompt;
}
