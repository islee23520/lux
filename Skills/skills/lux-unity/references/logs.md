# Lux Console Logs
Commands for reading and managing the Unity Editor console.

## lux unity get-logs
Retrieves the most recent entries from the Unity Console.

**Usage:**
```bash
lux unity get-logs
```

**Expected Output:**
JSON containing `totalCount`, `displayedCount`, and an array of `consoleLogs` with message, stack trace, and log type.

---

## lux unity clear-console
Clears all entries from the Unity Console.

**Usage:**
```bash
lux unity clear-console
```

**Expected Output:**
JSON containing `beforeCount` and `afterCount` (which should be 0).
