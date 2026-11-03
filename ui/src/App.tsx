import { useEffect, useRef, useState } from 'react'
import { ask, open, save } from '@tauri-apps/plugin-dialog'
import Palette from './panels/Palette'
import Logo from './brand/Logo'
import Canvas from './editor/Canvas'
import { buildCircuit, nodeLabels } from './editor/buildCircuit'
import { schematicFromCircuit } from './editor/fromCircuit'
import { useHistory } from './editor/history'
import { emptySchematic, updateValues, type Schematic } from './editor/schematic'
import { useMode, type Mode } from './modes/mode'
import type { PartDef } from './parts/registry'
import Properties from './panels/Properties'
import AcPanel from './panels/AcPanel'
import RunBar from './panels/RunBar'
import Scope from './panels/Scope'
import Readouts from './panels/Readouts'
import WarningBanner from './panels/WarningBanner'
import { damagedIds } from './panels/warningText'
import { glowLevels } from './panels/glow'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { loadCircuit, saveCircuit, type SimOutput } from './sim/api'
import './modes/theme.css'

const filters = [{ name: 'Nodal circuit', extensions: ['json'] }, { name: 'SPICE netlist', extensions: ['cir', 'sp'] }]
const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

function App() {
  const { mode, setMode } = useMode()
  const { state: schematic, set: setSchematic, undo, redo, canUndo, canRedo } = useHistory<Schematic>(emptySchematic())
  const saved = useRef<Schematic>(schematic) // the schematic as last saved or opened; undo back to it is clean again
  const dirty = schematic !== saved.current
  const [placing, setPlacing] = useState<PartDef | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [result, setResult] = useState<SimOutput | null>(null)
  const [node, setNode] = useState<number | null>(null)
  const [cursor, setCursor] = useState<number | null>(null)
  const [partId, setPartId] = useState<string | null>(null)
  // Result node indices and warnings belong to the schematic that was run.
  useEffect(() => setResult(null), [schematic])
  const part = schematic.parts.find((p) => p.id === partId)
  const labels = nodeLabels(schematic)
  const nodes = result?.node_voltages[0]?.length ?? 0
  const selected = node !== null && node < nodes ? node : Math.max(0, nodes - 1)
  const modes: [Mode, string][] = [['engineer', 'Engineer'], ['kids', 'Kids']]

  const dirtyRef = useRef(dirty)
  dirtyRef.current = dirty
  const confirmDiscard = () => ask('You have unsaved changes. Discard them?', { title: 'Nodal', kind: 'warning' })

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || (e.target as HTMLElement).closest('input, textarea, select')) return // leave text undo to the field
      const k = e.key.toLowerCase()
      if (k === 'z' && !e.shiftKey) undo()
      else if (k === 'y' || (k === 'z' && e.shiftKey)) redo()
      else return
      e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [undo, redo])

  useEffect(() => {
    if (!('__TAURI_INTERNALS__' in window)) return
    const off = getCurrentWindow().onCloseRequested(async (e) => {
      if (dirtyRef.current && !(await confirmDiscard())) e.preventDefault()
    })
    return () => { off.then((f) => f()) }
  }, [])

  const onSave = async () => {
    try {
      const path = await save({ filters })
      if (path) { await saveCircuit(path, buildCircuit(schematic)); saved.current = schematic }
      setError(null)
    } catch (e) {
      setError(`Could not save the circuit: ${message(e)}`)
    }
  }
  const onOpen = async () => {
    try {
      if (dirty && !(await confirmDiscard())) return
      const path = await open({ multiple: false, filters })
      if (typeof path === 'string') {
        const loaded = schematicFromCircuit(await loadCircuit(path))
        setSchematic(loaded)
        saved.current = loaded
      }
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
          <button type="button" onClick={undo} disabled={!canUndo}>Undo</button>
          <button type="button" onClick={redo} disabled={!canRedo}>Redo</button>
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
          <Canvas schematic={schematic} onChange={setSchematic} mode={mode} placing={placing} onPlaced={() => setPlacing(null)} damaged={damagedIds(result?.warnings, schematic)} glow={glowLevels(result, schematic)} onSelect={setPartId} />
          {part && <Properties key={part.id} part={part} mode={mode} onChange={(v) => setSchematic((s) => updateValues(s, part.id, v))} />}
          <WarningBanner warnings={result?.warnings} mode={mode} schematic={schematic} />
          <RunBar schematic={schematic} mode={mode} onResult={(o) => { setResult(o); setCursor(null) }} />
          {mode === 'engineer' && <AcPanel schematic={schematic} />}
          {result && (
            <section className="results">
              <label>Node{' '}
                <select aria-label="Node" value={selected} onChange={(e) => setNode(Number(e.target.value))}>
                  {Array.from({ length: nodes }, (_, i) => <option key={i} value={i}>{labels[i] ?? i}</option>)}
                </select>
              </label>
              <Scope output={result} selectedNode={selected} onCursor={setCursor} />
              <Readouts output={result} cursorTime={cursor} selectedNode={selected} mode={mode} nodeLabel={labels[selected]} />
            </section>
          )}
        </main>
      </div>
    </div>
  )
}

export default App
