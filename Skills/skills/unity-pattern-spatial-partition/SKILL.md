---
name: unity-pattern-spatial-partition
description: "Spatial Partition"
category: reference
source: unity-design-patterns-skills
source_path: patterns/spatial-partition/SKILL.md
---

# Spatial Partition

## Purpose
Organize objects by region so broad-phase queries inspect nearby candidates instead of the whole world.

## Use When
- Many repeated nearby-object queries exist.
- A grid, sectors, or tree can shrink the candidate set.
- Reindexing moving objects is affordable.

## Avoid When
- The world is small enough for full scans.
- Movement patterns make reindexing too expensive.

## Unity Implementation Recipe
1. Choose a partition that matches movement and query shape.
2. Implement insert, update, remove, and query explicitly.
3. Use the partition as broad-phase, not final truth.
4. Tune with real scene density.

## Common Pitfalls
- Poor cell sizing.
- Forgetting to update membership when objects move.

## Minimal Unity Sketch
```csharp
public sealed class SpatialGrid<T>
{
    private readonly Dictionary<Vector2Int, List<T>> _cells = new();

    public void Add(Vector2Int cell, T item)
    {
        if (!_cells.TryGetValue(cell, out var list)) _cells[cell] = list = new();
        list.Add(item);
    }
}
```

## Response Shape
- Say whether **Spatial Partition** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
