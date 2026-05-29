---
name: unity-pattern-strategy
description: "Strategy"
category: reference
source: unity-design-patterns-skills
source_path: patterns/strategy/SKILL.md
---

# Strategy

## Purpose
Swap algorithms behind a common interface so the caller depends on a contract instead of one hard-coded implementation.

## Use When
- The feature needs interchangeable runtime algorithms.
- Selection should be data-driven or injected.
- Algorithms should be tested independently.

## Avoid When
- There is only one stable algorithm.
- The variation is really mode-based behavior with transitions, which fits State better.

## Unity Implementation Recipe
1. Define minimal inputs and outputs.
2. Extract each algorithm behind one interface.
3. Centralize selection.
4. Keep strategies mostly stateless unless the domain needs state.

## Common Pitfalls
- Strategies depending on global singletons.
- Repeated strategy selection switches across many callers.

## Minimal Unity Sketch
```csharp
public interface ITargetingStrategy
{
    Enemy Select(IReadOnlyList<Enemy> enemies);
}

public sealed class LowestHealthTargeting : ITargetingStrategy
{
    public Enemy Select(IReadOnlyList<Enemy> enemies) => enemies.MinBy(e => e.Health)!;
}
```

## Response Shape
- Say whether **Strategy** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
