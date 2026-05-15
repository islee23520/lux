import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    environment: 'node',
    globals: false,
    include: ['**/*.test.ts'],
    passWithNoTests: true,
    setupFiles: ['./test/setup.ts'],
    clearMocks: true,
    restoreMocks: true,
    teardownTimeout: 5000,
    logHeapUsage: false,
    disableConsoleIntercept: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'text-summary', 'html', 'lcov'],
      include: ['**/*.ts'],
      exclude: [
        '**/*.test.ts',
        '**/*.d.ts',
        '**/node-shims.d.ts',
        '**/test/fixtures/**',
        '**/test/helpers/**',
      ],
      thresholds: {
        lines: 98,
        functions: 95,
        branches: 90,
        statements: 98,
      },
    },
  },
})
