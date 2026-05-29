---
name: unity-pattern-composite
description: "Composite"
category: reference
source: unity-design-patterns-skills
source_path: patterns/composite/SKILL.md
---

# Composite

## Purpose
Treat individual objects and groups through one interface so callers can ignore tree shape.

## Use When
- A feature naturally forms a tree.
- The same operation should work on a leaf or a group.
- Callers should not branch on single-versus-many.

## Avoid When
- The structure is shallow and explicit fields are clearer.
- Leaf and container behavior are too different for one contract.

## Unity Implementation Recipe
1. Define a small shared interface.
2. Implement leaves directly.
3. Let composites manage children and delegate recursion.
4. Document ownership and traversal cost.

## Common Pitfalls
- Forcing child-management methods onto leaves unnecessarily.
- Hiding expensive recursive work behind a cheap-looking method.

## Minimal Unity Sketch
```csharp
public interface IBuffNode
{
    void Apply(CharacterStats stats);
}

public sealed class BuffGroup : IBuffNode
{
    private readonly List<IBuffNode> _children = new();
    public void Add(IBuffNode node) => _children.Add(node);
    public void Apply(CharacterStats stats)
    {
        foreach (var child in _children) child.Apply(stats);
    }
}
```

## Response Shape
- Say whether **Composite** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
