import { useState } from 'react'
import type { LuxSkill } from '../types'

const LUX_SKILLS: LuxSkill[] = [
  { name: 'compile', description: 'Compile the Unity project', toolType: 'all' },
  { name: 'test', description: 'Run Unity EditMode tests', toolType: 'all' },
  { name: 'screenshot', description: 'Capture editor screenshot', toolType: 'all' },
  { name: 'logs', description: 'Get recent Unity console logs', toolType: 'all' },
  { name: 'playmode', description: 'Toggle Unity play mode', toolType: 'all' },
  { name: 'dynamic-code', description: 'Execute dynamic C# code', toolType: 'all' },
]

interface SkillPanelProps {
  onDispatchSkill: (skillName: string) => void
}

export function SkillPanel({ onDispatchSkill }: SkillPanelProps) {
  const [isOpen, setIsOpen] = useState(true)

  return (
    <div className={`skill-panel ${isOpen ? 'open' : 'closed'}`}>
      <div className="skill-panel-header" onClick={() => setIsOpen(!isOpen)}>
        <h3 className="eyebrow">Lux Skills</h3>
        <button className="toggle-btn">{isOpen ? '▼' : '▶'}</button>
      </div>
      
      {isOpen && (
        <div className="skill-grid">
          {LUX_SKILLS.map((skill) => (
            <button
              key={skill.name}
              className="skill-btn"
              onClick={() => onDispatchSkill(skill.name)}
              title={skill.description}
            >
              {skill.name}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
