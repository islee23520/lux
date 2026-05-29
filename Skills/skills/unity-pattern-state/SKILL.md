---
name: unity-pattern-state
description: "State"
category: reference
source: unity-design-patterns-skills
source_path: patterns/state/SKILL.md
---

# State

## Purpose
Move mode-specific behavior into dedicated state objects so a context stops growing giant conditionals.

## Use When
- Actors or flows behave differently by explicit mode.
- Entry or exit behavior belongs with each mode.
- Repeated switch statements are spreading.

## Avoid When
- There are only a couple of trivial branches.
- The real variation is just an interchangeable algorithm, which fits Strategy better.

## Unity Implementation Recipe
1. Define the context API.
2. Create a small state interface.
3. Move mode rules into concrete states.
4. Keep transitions explicit and traceable.

## Common Pitfalls
- Hidden transitions that are hard to follow.
- States reaching too deeply into context internals.

## Minimal Unity Sketch
```csharp
public interface IPlayerState
{
    void Enter(PlayerContext context);
}

public sealed class DeadState : IPlayerState
{
    public void Enter(PlayerContext context) => context.ShowGameOver();
}
```

## Response Shape
- Say whether **State** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
