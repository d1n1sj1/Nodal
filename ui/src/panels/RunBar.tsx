import { useRef, useState } from 'react'
import { buildCircuit } from '../editor/buildCircuit'
import type { Schematic } from '../editor/schematic'
import type { Mode } from '../modes/mode'
import { runSimulation, type SimOutput } from '../sim/api'
import { parseSI } from './formatSI'

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

interface Props { schematic: Schematic; onResult: (o: SimOutput) => void; mode?: Mode }

export default function RunBar({ schematic, onResult, mode = 'engineer' }: Props) {
  const [stop, setStop] = useState('10m')
  const [step, setStep] = useState('10u')
  const [error, setError] = useState<string | null>(null)
  const [running, setRunning] = useState(false)
  const latest = useRef(0)

  const run = async () => {
    const stopS = parseSI(stop)
    const stepS = parseSI(step)
    if (!(stopS > 0) || !(stepS > 0)) return setError('Enter positive times like 10m, 10u or 0.01.')
    const circuit = buildCircuit(schematic)
    if (circuit.parts.length === 0) return setError('Nothing to simulate yet: add some parts first.')
    const id = ++latest.current
    setError(null)
    setRunning(true)
    try {
      const out = await runSimulation(circuit, stopS, stepS)
      if (id === latest.current) onResult(out)
    } catch (e) {
      if (id === latest.current) setError(message(e))
    } finally {
      if (id === latest.current) setRunning(false)
    }
  }

  return (
    <div className="runbar">
      <label>Stop time <input aria-label="Stop time" value={stop} onChange={(e) => setStop(e.target.value)} size={7} /></label>
      <label>Step <input aria-label="Step" value={step} onChange={(e) => setStep(e.target.value)} size={7} /></label>
      <button type="button" className="run-btn" disabled={running} onClick={run}>{mode === 'kids' ? 'Play' : 'Run'}</button>
      {error && <span className="runbar-error" role="alert">{error}</span>}
    </div>
  )
}
