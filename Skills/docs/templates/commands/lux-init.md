---
description: Initialize or re-evaluate Lux workspace for the project (spec, glossary, domains)
subtask: true
---

Run `lux init --interactive` to initialize or re-evaluate the `.lux/` workspace for this project. This will:

1. Detect the Unity project settings (version, render pipeline, packages)
2. Create `.lux/spec.json` with detected project information when no spec exists
3. Load an existing `.lux/spec.json` when present and continue filling missing details
4. Start the interactive Socratic question loop to refine spec details
5. Generate any missing domain markdown files in `.lux/domains/`
6. Create `.lux/glossary.md` with project-specific terminology when missing

For a clean restart, run `lux init --force --interactive`. This backs up the existing `.lux/` workspace under `.lux/backups/reinit-*` before creating fresh state.

Execute the command:

```bash
lux init --interactive
```

After the command completes, read `.lux/spec.json` and summarize the initialized project configuration to the user.
