export interface NextActionContext {
  // Ticket state
  activeTickets: Array<{ id?: string; title?: string; status: string; priority?: string; spec_ref?: string | null }>
  inactiveTickets: Array<{ id?: string; title?: string; status: string }>
  ticketCounts: Record<string, number>
  incompleteCount: number
  
  // Spec state
  ambiguityScore: number
  shouldContinueSpec: boolean
  nextSpecAction: string
  
  // Continuation state
  continuationCount: number
  stagnationCount: number
  consecutiveFailures: number
  lastAmbiguity: number
  
  // Optional external signals
  healthScore?: number          // 0-100 from T10
  lastError?: string            // from T10
  suggestedAction?: string       // from T10
  
  // Session info
  isCompactionGuardActive: boolean
  maxContinuations: number
}

export interface NextActionResult {
  message: string              // The actual prompt text to inject
  shouldInject: boolean         // Whether to inject at all
  confidence: number             // 0-1 how confident we are in this action
  reason: string               // Why this action was chosen
}

function calculateConfidence(ctx: NextActionContext): number {
  // Higher confidence when:
  // - There are clear active tickets (+0.3)
  // - Ambiguity is low (+0.2)
  // - Health score is good (+0.2)
  // - Stagnation is low (+0.2)
  // - This isn't the last continuation (+0.1)
  let conf = 0.1 // base confidence
  
  if (ctx.activeTickets.length > 0) conf += 0.3
  if (ctx.ambiguityScore < 0.3) conf += 0.2
  if ((ctx.healthScore ?? 100) >= 70) conf += 0.2
  if (ctx.stagnationCount === 0) conf += 0.2
  if (ctx.continuationCount < Math.floor(ctx.maxContinuations * 0.7)) conf += 0.1
  
  return Math.min(1, Math.round(conf * 100) / 100)
}

function determineReason(ctx: NextActionContext): string {
  if (ctx.healthScore !== undefined && ctx.healthScore < 40) return "low-health-score"
  if (ctx.stagnationCount >= 3) return "stagnation-recovery"
  if (ctx.activeTickets.length > 0) return "active-tickets"
  if (ctx.incompleteCount > 0) return "incomplete-tickets"
  return "spec-continuation"
}

function allDoneMessage(_ctx: NextActionContext): string {
  return `[Lux] All spec tickets complete!

🎉 Spec-driven implementation finished.
Run \`lux verify\` to validate final state, then update spec ambiguity scores.
No further continuation needed — awaiting user direction.`
}

function maxContinuationsMessage(ctx: NextActionContext): string {
  return `[Lux] Maximum continuations reached (${ctx.maxContinuations}). Current spec ambiguity: review and update manually, or start a new session to continue.`
}

export function generateNextAction(ctx: NextActionContext): NextActionResult {
  const { activeTickets, incompleteCount, ambiguityScore, stagnationCount,
          healthScore, suggestedAction, continuationCount } = ctx
  
  // Edge case: nothing to do
  if (incompleteCount === 0 && activeTickets.length === 0) {
    return {
      message: allDoneMessage(ctx),
      shouldInject: false,
      confidence: 0.95,
      reason: "all-tickets-complete",
    }
  }
  
  // Edge case: max continuations reached
  if (continuationCount >= ctx.maxContinuations) {
    return {
      message: maxContinuationsMessage(ctx),
      shouldInject: false,
      confidence: 1.0,
      reason: "max-continuations-reached",
    }
  }
  
  // Edge case: compaction guard active
  if (ctx.isCompactionGuardActive) {
    return {
      message: "",
      shouldInject: false,
      confidence: 0,
      reason: "compaction-guard-active",
    }
  }
  
  // Main path: build contextual prompt
  const sections: string[] = []
  
  // Header
  sections.push("[Lux] Continue spec-driven implementation.")
  
  // Priority-ordered ticket list (if any active)
  if (activeTickets.length > 0) {
    sections.push("\nCurrent focus (priority order):")
    for (let i = 0; i < Math.min(activeTickets.length, 8); i++) {
      const t = activeTickets[i]
      sections.push(`${i + 1}. [${t.status}] ${t.title || t.id}${t.spec_ref ? ` (${t.spec_ref})` : ""}`)
    }
  }
  
  // Spec context
  sections.push(`\nSpec ambiguity: ${Math.round(ambiguityScore * 100)}%`)
  if (ctx.nextSpecAction && ctx.shouldContinueSpec) {
    sections.push(`Spec recommendation: ${ctx.nextSpecAction}`)
  }
  
  // Continuation metrics
  sections.push(`Continuation ${continuationCount}/${ctx.maxContinuations} — Stagnation: ${stagnationCount}, Failures: ${ctx.consecutiveFailures}`)
  
  // External signal integration (if available)
  if (healthScore !== undefined && healthScore < 50) {
    sections.push(`⚠️ Build health: ${healthScore}/100 — ${suggestedAction || "Check recent errors"}`)
  }
  
  // Next action suggestion
  const nextTicket = activeTickets[0]
  if (nextTicket) {
    sections.push(`\nNext action: Continue with "${nextTicket.title || nextTicket.id}". Update status to Done when complete.`)
  } else if (suggestedAction) {
    sections.push(`\nSuggested: ${suggestedAction}`)
  }
  
  // Footer
  sections.push("\nDo not ask for permission. Use .lux as source of truth. Update ticket/spec state as progress is made.")
  
  const message = sections.join("\n")
  
  return {
    message,
    shouldInject: true,
    confidence: calculateConfidence(ctx),
    reason: determineReason(ctx),
  }
}
