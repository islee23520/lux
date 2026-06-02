import { describe, expect, it } from "vitest";
import { buildContinuationPrompt, PromptBuilderContext } from "../prompt-builder";
import { Ticket, TicketSummary } from "../ticket-loader";

describe("prompt-builder", () => {
  const mockTicket: Ticket = {
    id: "T-101",
    title: "Test Ticket",
    spec_ref: "SPEC-1",
    status: "Todo",
    acceptance_criteria: ["Criteria 1", "Criteria 2"],
    description: "Test description",
    priority: "High",
    type: "Feature",
  };

  const mockSummary: TicketSummary = {
    tickets: [
      { id: "T-101", status: "ToDo" },
      { id: "T-102", status: "Done" },
    ],
    byStatus: { ToDo: 1, Done: 1 },
    activeTickets: [{ id: "T-101", status: "ToDo" }],
    incompleteCount: 1,
  };

  const defaultCtx: PromptBuilderContext = {
    ticket: mockTicket,
    nextAction: {
      message: "Next step message",
      confidence: 0.95,
      shouldInject: true,
      reason: "test",
    },
    ambiguity: 0.2,
    summary: mockSummary,
    continuationCount: 0,
    consecutiveFailures: 0,
    aiContext: {
      ontology: {
        schemaVersion: "1.0.0",
        requiredTerms: [
          "scene",
          "stage",
          "actor",
          "component",
          "transform",
          "camera",
          "viewport",
          "coordinate_frames",
          "expected_visual_state",
          "evidence_class",
          "blocker_class",
          "completion_gate",
          "schema_version",
        ],
      },
      astSummary: {
        source: "scene",
        nodeCount: 3,
        nodeTypes: ["GameObject", "Component"],
      },
      coordinateMappingSummary: {
        frames: ["world", "local", "screen", "viewport", "ui"],
        origins: ["player_spawn"],
      },
      evidenceGateRequirements: {
        requiredEvidence: ["scene_ast", "coordinate_map", "expected_visual_state", "vision_match"],
        requiredReferences: ["ast_node", "coordinate_region", "contract_doc", "blocker_reason"],
      },
      blockers: [{ kind: "dirty_worktree", reason: "working tree not clean" }],
    },
  };

  describe("buildContinuationPrompt", () => {
    it("includes AI context sections for V13 injection", () => {
      const prompt = buildContinuationPrompt(defaultCtx);

      expect(prompt).toContain("AI context:");
      expect(prompt).toContain("Ontology: schema=1.0.0; required=scene, stage, actor");
      expect(prompt).toContain("AST summary: source=scene; nodes=3; types=GameObject, Component");
      expect(prompt).toContain("Coordinate mapping: frames=world, local, screen, viewport, ui; origins=player_spawn");
      expect(prompt).toContain("Evidence gates: evidence=scene_ast, coordinate_map, expected_visual_state, vision_match; references=ast_node, coordinate_region, contract_doc, blocker_reason");
      expect(prompt).toContain("Blockers: dirty_worktree (working tree not clean)");
      expect(prompt).toContain("Constraints: pixel-only completion is forbidden. Fake engine parity is forbidden.");
    });

    it("builds a standard continuation prompt", () => {
      const prompt = buildContinuationPrompt(defaultCtx);

      expect(prompt).toContain("[Lux] Continue spec-driven implementation. Next step message");
      expect(prompt).toContain("Current focus: [T-101] Test Ticket (SPEC-1)");
      expect(prompt).toContain("Acceptance criteria:");
      expect(prompt).toContain("- [ ] Criteria 1");
      expect(prompt).toContain("- [ ] Criteria 2");
      expect(prompt).toContain("Progress: 1/2 tickets complete. Continuation #1.");
      expect(prompt).toContain("Spec ambiguity: 20%. Decision confidence: 0.95.");
      expect(prompt).toContain("AI context:");
      expect(prompt).toContain("Ontology: schema=1.0.0; required=scene, stage, actor");
      expect(prompt).toContain("AST summary: source=scene; nodes=3; types=GameObject, Component");
      expect(prompt).toContain("Coordinate mapping: frames=world, local, screen, viewport, ui; origins=player_spawn");
      expect(prompt).toContain("Evidence gates: evidence=scene_ast, coordinate_map, expected_visual_state, vision_match; references=ast_node, coordinate_region, contract_doc, blocker_reason");
      expect(prompt).toContain("Blockers: dirty_worktree (working tree not clean)");
      expect(prompt).toContain("Constraints: pixel-only completion is forbidden. Fake engine parity is forbidden.");
      expect(prompt).toContain("Preserve .lux as the source of truth.");
    });

    it("handles consecutive failures with error message", () => {
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        consecutiveFailures: 2,
        lastError: "Compilation failed",
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt).toContain("Previous failures detected: 2. Last error: Compilation failed.");
      expect(prompt).toContain("Prioritize fixing the failing path before broadening scope.");
    });

    it("handles consecutive failures without error message", () => {
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        consecutiveFailures: 1,
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt).toContain("Previous failures detected: 1.");
      expect(prompt).not.toContain("Last error:");
    });

    it("formats acceptance criteria from a newline-separated string", () => {
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        ticket: {
          ...mockTicket,
          acceptance_criteria: "Line 1\nLine 2",
        },
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt).toContain("- [ ] Line 1");
      expect(prompt).toContain("- [ ] Line 2");
    });

    it("cleans existing prefixes from acceptance criteria", () => {
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        ticket: {
          ...mockTicket,
          acceptance_criteria: ["- [ ] Already prefixed", "* Bullet point", "- Already dashed"],
        },
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt).toContain("- [ ] Already prefixed");
      expect(prompt).toContain("- [ ] Bullet point");
      expect(prompt).toContain("- [ ] Already dashed");
      // Ensure we don't double prefix
      expect(prompt).not.toContain("- [ ] - [ ]");
    });

    it("provides default message for empty acceptance criteria", () => {
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        ticket: {
          ...mockTicket,
          acceptance_criteria: [],
        },
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt).toContain("- [ ] Complete the ticket according to its spec reference.");
    });

    it("handles missing ticket metadata", () => {
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        ticket: {
          ...mockTicket,
          id: undefined as any,
          title: undefined as any,
          spec_ref: undefined as any,
        },
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt).toContain("Current focus: [unknown] Untitled ticket (no spec_ref)");
    });

    it("rounds ambiguity score", () => {
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        ambiguity: 0.1234,
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt).toContain("Spec ambiguity: 12%.");
    });

    it("truncates long prompts", () => {
      const longCriteria = Array(100).fill("Very long acceptance criteria line that takes up space").join("\n");
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        ticket: {
          ...mockTicket,
          acceptance_criteria: longCriteria,
        },
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt.length).toBeLessThanOrEqual(2000);
      expect(prompt.endsWith("...")).toBe(true);
    });

    it("increments continuation count in display", () => {
      const ctx: PromptBuilderContext = {
        ...defaultCtx,
        continuationCount: 5,
      };
      const prompt = buildContinuationPrompt(ctx);

      expect(prompt).toContain("Continuation #6.");
    });
  });
});
