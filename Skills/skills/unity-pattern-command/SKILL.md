---
name: unity-pattern-command
description: "Command"
category: reference
source: unity-design-patterns-skills
source_path: patterns/command/SKILL.md
---

# Command

## Purpose
Represent an action as an object so execution can be decoupled from input, scheduling, history, and replay.

## Use When
- The same action should be triggered by input, AI, or UI.
- You need undo, replay, logging, or queuing.
- Callers should not know the receiver details.

## Avoid When
- A direct method call is enough.
- You would create commands with hidden dependencies on broad global state.

## Unity Implementation Recipe
1. Create a small command interface.
2. Keep invoker and receiver separate.
3. Capture only state needed for execute or undo.
4. Add history or queuing outside the command body.

## Common Pitfalls
- Commands that capture stale mutable references.
- One mega-command that handles unrelated actions.

## Minimal Unity Sketch
```csharp
public interface ICommand
{
    void Execute();
}

public sealed class JumpCommand : ICommand
{
    private readonly PlayerMotor _motor;
    public JumpCommand(PlayerMotor motor) => _motor = motor;
    public void Execute() => _motor.Jump();
}
```

## Response Shape
- Say whether **Command** is actually the right fit.
- Map it to concrete Unity types, ownership, and lifecycle.
- Prefer a production-shaped skeleton over a long tutorial dump.
- Call out the simpler alternative if this pattern would be overengineering.
