# Lux Compile and Tests
Commands for batch-mode compilation and test execution.

## lux compile
Compiles the Unity project through Lux batch mode.

**Usage:**
```bash
lux compile
```

**Expected Output:**
JSON indicating success or failure, with a path to the compilation log.

---

## lux run-tests
Executes Unity Test Runner for EditMode or PlayMode tests.

**Flags:**
- `--test-platform <platform>`: `EditMode` (default) or `PlayMode`.
- `--test-results <path>`: Optional path to save XML results.
- `--log-file <path>`: Optional path for the test log.

**Usage:**
```bash
# Run EditMode tests
lux run-tests --test-platform EditMode

# Run PlayMode tests
lux run-tests --test-platform PlayMode
```

**Expected Output:**
JSON containing test results summary and paths to XML results and logs.
