import { useState } from 'react'
import { open, save } from '@tauri-apps/plugin-dialog'
import Palette from './panels/Palette'
import Logo from './brand/Logo'
import Canvas from './editor/Canvas'
import { buildCircuit } from './editor/buildCircuit'
import { schematicFromCircuit } from './editor/fromCircuit'
import { emptySchematic, type Schematic } from './editor/schematic'
import { useMode, type Mode } from './modes/mode'
import type { PartDef } from './parts/registry'
import RunBar from './panels/RunBar'
import Scope from './panels/Scope'
import Readouts from './panels/Readouts'
import { loadCircuit, saveCircuit, type SimOutput } from './sim/api'
import './modes/theme.css'

const filters = [{ name: 'Nodal circuit', extensions: ['json'] }]
const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

function App() {
  const { mode, setMode } = useMode()
  const [schematic, setSchematic] = useState<Schematic>(emptySchematic)
  const [placing, setPlacing] = useState<PartDef | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [result, setResult] = useState<SimOutput | null>(null)
  const [node, setNode] = useState<number | null>(null)
  const [cursor, setCursor] = useState<number | null>(null)
  const nodes = result?.node_voltages[0]?.length ?? 0
  const selected = node !== null && node < nodes ? node : Math.max(0, nodes - 1)
  const modes: [Mode, string][] = [['engineer', 'Engineer'], ['kids', 'Kids']]

  const onSave = async () => {
    try {
      const path = await save({ filters })
      if (path) await saveCircuit(path, buildCircuit(schematic))
      setError(null)
    } catch (e) {
      setError(`Could not save the circuit: ${message(e)}`)
    }
  }
  const onOpen = async () => {
    try {
      const path = await open({ multiple: false, filters })
      if (typeof path === 'string') setSchematic(schematicFromCircuit(await loadCircuit(path)))
      setError(null)
    } catch (e) {
      setError(`Could not open that file: ${message(e)}`)
    }
  }

  return (
    <div className="app" data-mode={mode}>
      <header className="app-header">
        <span className="brand">
          <Logo />
          {mode === 'kids' && <span className="kids-tag">Kids</span>}
        </span>
        <div className="file-actions">
          <button type="button" onClick={onOpen}>Open</button>
          <button type="button" onClick={onSave}>Save</button>
        </div>
        <div className="mode-switch" role="group" aria-label="Mode">
          {modes.map(([m, label]) => (
            <button key={m} type="button" aria-pressed={mode === m} onClick={() => setMode(m)}>
              {label}
            </button>
          ))}
        </div>
      </header>
      <div className="app-body">
        <Palette mode={mode} onPick={setPlacing} />
        <main className="app-main">
          {error && <p className="app-error" role="alert">{error}</p>}
          <Canvas schematic={schematic} onChange={setSchematic} mode={mode} placing={placing} onPlaced={() => setPlacing(null)} />
          <RunBar schematic={schematic} mode={mode} onResult={(o) => { setResult(o); setCursor(null) }} />
          {result && (
            <section className="results">
              <label>Node{' '}
                <select aria-label="Node" value={selected} onChange={(e) => setNode(Number(e.target.value))}>
                  {Array.from({ length: nodes }, (_, i) => <option key={i} value={i}>{i}</option>)}
                </select>
              </label>
              <Scope output={result} selectedNode={selected} onCursor={setCursor} />
              <Readouts output={result} cursorTime={cursor} selectedNode={selected} mode={mode} />
            </section>
          )}
        </main>
      </div>
    </div>
  )
}

export default App
