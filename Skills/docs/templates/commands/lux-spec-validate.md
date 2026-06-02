---
description: Validate the Lux specs contract and report any issues
subtask: true
---

Validate the current `.lux/specs/spec.json` configuration and report any spec errors or missing fields. Treat `.lux/spec.json` only as a legacy compatibility fallback.

Execute:

```bash
lux spec validate
```

Report the validation results to the user. If there are errors, suggest fixes.
