import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  base: process.env.VITE_BASE ?? '/ui/',
  build: {
    outDir: '../ui',
    emptyOutDir: true,
  },
  server: {
    watch: {
      ignored: ['**/Skills/**', '**/.lux/skills/**'],
    },
  },
  plugins: [
    react(),
    tailwindcss(),
  ],
})
