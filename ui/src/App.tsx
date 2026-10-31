import Palette from './panels/Palette'
import Logo from './brand/Logo'
import { useMode, type Mode } from './modes/mode'
import './modes/theme.css'

function App() {
  const { mode, setMode } = useMode()
  const modes: [Mode, string][] = [['engineer', 'Engineer'], ['kids', 'Kids']]
  return (
    <div className="app" data-mode={mode}>
      <header className="app-header">
        <span className="brand">
          <Logo />
          {mode === 'kids' && <span className="kids-tag">Kids</span>}
        </span>
        <div className="mode-switch" role="group" aria-label="Mode">
          {modes.map(([m, label]) => (
            <button key={m} type="button" aria-pressed={mode === m} onClick={() => setMode(m)}>
              {label}
            </button>
          ))}
        </div>
      </header>
      <div className="app-body">
        <Palette mode={mode} onPick={() => {}} />
        <main className="app-main" />
      </div>
    </div>
  )
}

export default App
