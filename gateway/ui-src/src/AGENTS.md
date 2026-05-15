# React 19 SPA Dashboard

Vite + React 19 + TypeScript strict. Web dashboard for LUX gateway.

## STRUCTURE
```
ui-src/src/
├── App.tsx                    # Router: /, /compile, /test, /log, /skills, /project, /sessions, /tools
├── main.tsx                   # createRoot bootstrap
├── components/
│   ├── dashboard/             # DashboardLayout, StatsCard, ServerStatus
│   ├── sidebar/               # Sidebar navigation
│   ├── CompilePanel.tsx       # Compile trigger + results
│   ├── TestPanel.tsx          # Test runner UI
│   ├── LogPanel.tsx           # AI interaction log viewer
│   ├── SkillsPanel.tsx        # Skill browser
│   ├── ProjectPanel.tsx       # Unity project info
│   ├── SessionManager.tsx     # Tool/Remote session tabs
│   └── ToolSelector.tsx       # AI tool switcher (Claude/Code/OpenCode)
├── hooks/
│   ├── useDashboard.ts        # /api/health polling (10s interval)
│   ├── useCompile.ts          # /api/compile hook
│   ├── useTest.ts             # /api/test hook
│   ├── useSkills.ts           # /api/skills hook
│   ├── useLog.ts              # /api/log hook
│   └── useProject.ts          # /api/project hook
└── assets/                    # SVG icons
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add new page/panel | `components/` + `App.tsx` route | Create component, add Route |
| Fix API hook | `hooks/` | No mock/fallback data allowed |
| Fix sidebar | `components/sidebar/` | Navigation items |
| Server status | `hooks/useDashboard.ts` | Polls `/api/health` |
| Session management | `components/SessionManager.tsx` | Tool Sessions + Remote tabs |

## CONVENTIONS
- Functional components + hooks only. No class components.
- TypeScript strict mode. No `as any`, `@ts-ignore`.
- API hooks never return mock/fallback data.
- Built assets go to `gateway/ui/` (gitignored). Source is always `ui-src/`.

## COMMANDS
```bash
cd gateway/ui-src && npx tsc --noEmit
cd gateway/ui-src && npm run build
```
