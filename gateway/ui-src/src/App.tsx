import { useState, type ReactNode } from 'react'
import { BrowserRouter, Routes, Route, Navigate, useNavigate } from 'react-router-dom'
import 'reactflow/dist/style.css'
import 'xterm/css/xterm.css'
import './App.css'
import './components/dashboard/dashboard.css'
import { DashboardLayout } from './components/dashboard/DashboardLayout'
import { DashboardOverview } from './components/dashboard/DashboardOverview'
import { CompilePanel } from './components/dashboard/CompilePanel'
import { TestPanel } from './components/dashboard/TestPanel'
import { LogPanel } from './components/dashboard/LogPanel'
import { SkillMarketplace } from './components/dashboard/SkillMarketplace'
import { SessionManager } from './components/SessionManager'
import { ToolSelector } from './components/ToolSelector'
import { UnityRunPanel } from './components/UnityRunPanel'
import { ProjectSelector } from './components/ProjectSelector'
import { KanbanBoard } from './components/kanban/KanbanBoard'
import { SpecViewer } from './components/spec/SpecViewer'
import { ProgressGraphs } from './components/progress/ProgressGraphs'
import { TerminalPanel } from './components/terminal/TerminalPanel'
import { WebGLPlayer } from './components/player/WebGLPlayer'
import { PlayLogViewer } from './components/logs/PlayLogViewer'
import { LoopControlPanel } from './components/lux/LoopControlPanel'
import { useDashboard } from './hooks/useDashboard'
import type { ProjectInfo } from './hooks/useDashboard'
import type { ToolSession } from './types'

const routerBaseName = import.meta.env.BASE_URL.startsWith('/')
  ? import.meta.env.BASE_URL.replace(/\/$/, '')
  : undefined

// Wrapper component to provide navigation to DashboardOverview
function DashboardOverviewWrapper() {
  const navigate = useNavigate();
  const { projectInfo } = useDashboard();
  
  if (!projectInfo) {
    return (
      <section className="flex items-center justify-center h-full w-full p-8" aria-label="Project selection">
        <ProjectSelector onProjectAttached={() => window.location.reload()} />
      </section>
    );
  }
  
  return <DashboardOverview projectInfo={projectInfo} onNavigate={(panel) => navigate(`/${panel}`)} />;
}

function ProjectRoute({ children }: { children: (projectInfo: ProjectInfo | null) => ReactNode }) {
  const { projectInfo } = useDashboard()
  return <>{children(projectInfo)}</>
}

function App() {
  const [activeTool, setActiveTool] = useState<string>('claude')
  const [sessions] = useState<Map<string, ToolSession>>(new Map())

  return (
    <BrowserRouter basename={routerBaseName}>
      <Routes>
        <Route path="/" element={<DashboardLayout />}>
          <Route index element={<Navigate to="/dashboard" replace />} />
          <Route path="dashboard" element={<DashboardOverviewWrapper />} />
          <Route path="compile" element={<CompilePanel />} />
          <Route path="test" element={<TestPanel />} />
          <Route path="log" element={<LogPanel />} />
          <Route path="skills" element={<SkillMarketplace />} />
          <Route path="sessions" element={<SessionManager onSessionSelect={(id) => console.log('Selected session:', id)} />} />
          <Route path="tools" element={<ToolSelector activeTool={activeTool} onSelectTool={setActiveTool} sessions={sessions} />} />
          <Route path="unity-run" element={<UnityRunPanel />} />
          <Route path="workbench" element={<ProjectRoute>{(projectInfo) => <SpecViewer projectPath={projectInfo?.path ?? null} />}</ProjectRoute>} />
          <Route path="specs" element={<Navigate to="/workbench" replace />} />
          <Route path="kanban" element={<ProjectRoute>{(projectInfo) => <KanbanBoard projectPath={projectInfo?.path ?? ''} />}</ProjectRoute>} />
          <Route path="progress" element={<ProjectRoute>{(projectInfo) => <ProgressGraphs projectPath={projectInfo?.path} />}</ProjectRoute>} />
          <Route path="terminal" element={<TerminalPanel />} />
          <Route path="play" element={<ProjectRoute>{(projectInfo) => <WebGLPlayer projectPath={projectInfo?.path} />}</ProjectRoute>} />
          <Route path="logs" element={<ProjectRoute>{(projectInfo) => <PlayLogViewer projectPath={projectInfo?.path ?? ''} />}</ProjectRoute>} />
          <Route path="loop-control" element={<ProjectRoute>{(projectInfo) => <LoopControlPanel projectPath={projectInfo?.path ?? null} />}</ProjectRoute>} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}

export default App
