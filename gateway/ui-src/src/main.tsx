import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './assets/design-tokens.css'
import './assets/effects.css'
import './assets/typography.css'
import './assets/animations.css'
import './assets/lux-layout.css'
import './index.css'
import App from './App.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
