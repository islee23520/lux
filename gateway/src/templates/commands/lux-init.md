---
description: Initialize or re-evaluate Lux workspace for the project (spec, glossary, game domains)
subtask: true
---

Run `lux init --interactive` to initialize or re-evaluate the `.lux/` workspace for this project. This will:

1. Detect the Unity project settings (version, render pipeline, packages)
2. Create canonical `.lux/specs/spec.json` with detected project information when no spec exists
3. Load `.lux/specs/spec.json` when present, using `.lux/spec.json` only as a legacy compatibility fallback
4. Start the interactive Socratic question loop to refine spec details
5. Generate any missing domain markdown files in `.lux/specs/domains/`
6. Create `.lux/glossary.md` with project-specific terminology when missing

For a clean restart, run `lux init --force --interactive`. This backs up the existing `.lux/` workspace under `.lux/backups/reinit-*` before creating fresh state.

Execute the command:

```bash
lux init --interactive
```

After the command completes, read `.lux/specs/spec.json` and summarize the initialized project configuration to the user.
