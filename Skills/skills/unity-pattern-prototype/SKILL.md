---
name: unity-pattern-prototype
description: "Prototype"
category: reference
source: unity-design-patterns-skills
source_path: patterns/prototype/SKILL.md
---

# Prototype

## Purpose
Create new objects by cloning a configured prototype and customizing only what differs.

## Use When
- A base template already captures most configuration.
- Authors should tune reusable prototypes in the editor.
- Copy-then-adjust is clearer than many factory branches.

## Avoid When
- Deep-copy rules are unclear.
- Scene references or identity values cannot be copied safely.

## Unity Implementation Recipe
1. Define clone boundaries and deep-copy rules.
2. Keep author-time template data separate from runtime identity.
3. Apply small overrides after cloning.
4. Test clone independence for nested mutable data.

## Common Pitfalls
- Shared mutable data between original and clone.
- Copying runtime IDs or event subscriptions.

## Minimal Unity Sketch
```csharp
public sealed class EnemyPrototype
{
    public EnemyStats Stats;
    public EnemyPrototype Clone()
    {
        return new EnemyPrototype
        {
            Stats = Stats with { }
        };
    }
}
```

## Response Shape
- Say whether **Prototype** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
