# Lux Dynamic Code
Commands for executing dynamic C# snippets in the Unity Editor.

## lux unity execute-dynamic-code
Compiles and executes a C# snippet on the Unity main thread.

**Flags:**
- `--code <code>`: The C# code string to execute.
- `--file <path>`: Path to a file containing the C# code.

**Usage:**
```bash
# Execute from string
lux unity execute-dynamic-code --code "Debug.Log(\"Hello from Lux\");"

# Execute from file
lux unity execute-dynamic-code --file scripts/my-script.cs
```

**Safety Policy:**
Dynamic code is subject to the Lux Dynamic Code Safety Policy. Direct filesystem access (`System.IO`) and asset modification are generally blocked.

**Expected Output:**
JSON containing `success`, `result` (if any), `resultType`, and any `logs` or `diagnostics` emitted during execution.
