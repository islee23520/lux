---
description: Run full Lux game harness verification
subtask: true
---

Run the full Lux game harness verification suite for the current project.

Execute:

```bash
lux verify
```

This will:
1. Validate the spec configuration
2. Check Unity project structure
3. Verify build artifacts
4. Run any configured test suites

Report verification results to the user. If any checks fail, provide actionable next steps.