---
name: unity-pattern-memento
description: "Memento"
category: reference
source: unity-design-patterns-skills
source_path: patterns/memento/SKILL.md
---

# Memento

## Purpose
Capture object state and restore it later without exposing every internal detail to outside callers.

## Use When
- You need checkpoints or local undo.
- Rollback is clearer than replaying every command.
- The originator should control its own restore behavior.

## Avoid When
- Snapshots would be too large or too frequent.
- A command log or deterministic recomputation is cheaper and clearer.

## Unity Implementation Recipe
1. Choose the originator boundary.
2. Store only durable state in the memento.
3. Expose create and restore methods on the originator.
4. Version snapshots if they survive across sessions.

## Common Pitfalls
- Accidental shallow copies.
- Putting transient scene references into snapshots.

## Minimal Unity Sketch
```csharp
public readonly record struct PlayerMemento(int Health, Vector3 Position);

public sealed class PlayerState
{
    public int Health { get; private set; }
    public Vector3 Position { get; private set; }

    public PlayerMemento Save() => new(Health, Position);
    public void Restore(PlayerMemento memento)
    {
        Health = memento.Health;
        Position = memento.Position;
    }
}
```

## Response Shape
- Say whether **Memento** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
