---
name: unity-pattern-mediator
description: "Mediator"
category: reference
source: unity-design-patterns-skills
source_path: patterns/mediator/SKILL.md
---

# Mediator

## Purpose
Move many-to-many coordination rules into one place so peers stop depending on each other directly.

## Use When
- Several UI or gameplay objects coordinate frequently.
- Interaction logic changes more often than participant logic.
- A flow needs one authority over cross-object decisions.

## Avoid When
- Only two objects communicate and a direct link is clearer.
- A broadcast event is enough and no coordinator is needed.

## Unity Implementation Recipe
1. List participants and interaction rules.
2. Create mediator verbs around meaningful domain events.
3. Let colleagues report to the mediator instead of controlling peers.
4. Keep lifetime and ownership explicit.

## Common Pitfalls
- The mediator turning into a god object.
- Hiding event order so debugging gets harder.

## Minimal Unity Sketch
```csharp
public sealed class CombatMediator
{
    public void ReportEnemyDefeated(Enemy enemy, ScoreView scoreView, LootService loot)
    {
        scoreView.Add(enemy.ScoreValue);
        loot.Roll(enemy.LootTable);
    }
}
```

## Response Shape
- Say whether **Mediator** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
