# Lux Backend Status
Commands for checking the health and capabilities of the live Unity AI Bridge backend.

## lux unity backend-status
Checks whether the live Lux backend is available and responding to pings.

**Usage:**
```bash
lux unity backend-status
```

**Expected Output:**
JSON containing `ok`, `running`, `host`, `port`, and a `ping` response envelope.

---

## lux unity backend-list-commands
Lists all automation commands currently registered in the Unity backend.

**Usage:**
```bash
lux unity backend-list-commands
```

**Expected Output:**
JSON containing `schemaVersion`, `backendVersion`, and a `commands` array of strings.
