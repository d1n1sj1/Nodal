import { useState } from 'react'
import { buildCircuit, nodeLabels } from '../editor/buildCircuit'
import type { Schematic } from '../editor/schematic'
import { runAc, type AcOutput } from '../sim/api'
import Bode from './Bode'
import { parseSI } from './formatSI'

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

/** Small-signal frequency sweep around the DC operating point. One voltage source is the 1 V AC stimulus. */
export default function AcPanel({ schematic }: { schematic: Schematic }) {
  const [source, setSource] = useState('')
  const [start, setStart] = useState('10')
  const [stop, setStop] = useState('100k')
  const [ppd, setPpd] = useState('20')
  const [node, setNode] = useState(1)
  const [ran, setRan] = useState<{ out: AcOutput; of: Schematic } | null>(null)
  const out = ran && ran.of === schematic ? ran.out : null // any edit makes the plot stale
  const [error, setError] = useState<string | null>(null)
  const [running, setRunning] = useState(false)

  const sources = schematic.parts.filter((p) => p.def.kind === 'VSource')
  const chosen = sources.some((p) => p.id === source) ? source : sources[0]?.id
  const labels = nodeLabels(schematic)

  const run = async () => {
    const [f0, f1, n] = [parseSI(start), parseSI(stop), parseSI(ppd)]
    if (!chosen) return setError('Add a voltage source first: it is the signal the sweep sends in.')
    if (!(f0 > 0) || !(f1 > f0)) return setError('Enter frequencies like 10 and 100k, with start below stop.')
    if (!(n >= 1) || !Number.isInteger(n)) return setError('Points per decade must be a whole number, like 20.')
    setError(null)
    setRunning(true)
    try {
      setRan({ out: await runAc(buildCircuit(schematic), chosen, f0, f1, n), of: schematic })
    } catch (e) {
      setRan(null)
      setError(message(e))
    } finally {
      setRunning(false)
    }
  }

  return (
    <section className="acpanel" aria-label="AC sweep">
      <div className="runbar">
        <strong>AC sweep</strong>
        <label>Source{' '}
          <select aria-label="AC source" value={chosen ?? ''} onChange={(e) => setSource(e.target.value)}>
            {sources.map((p) => <option key={p.id} value={p.id}>{p.id}</option>)}
          </select>
        </label>
        <label>From <input aria-label="Start frequency" value={start} onChange={(e) => setStart(e.target.value)} size={6} /></label>
        <label>To <input aria-label="Stop frequency" value={stop} onChange={(e) => setStop(e.target.value)} size={6} /></label>
        <label>Points/decade <input aria-label="Points per decade" value={ppd} onChange={(e) => setPpd(e.target.value)} size={4} /></label>
        <button type="button" className="run-btn" disabled={running} onClick={run}>Sweep</button>
        {error && <span className="runbar-error" role="alert">{error}</span>}
      </div>
      {out && (
        <div className="results">
          <label>Node{' '}
            <select aria-label="AC node" value={Math.min(node, labels.length - 1)} onChange={(e) => setNode(Number(e.target.value))}>
              {labels.map((l, i) => <option key={i} value={i}>{l}</option>)}
            </select>
          </label>
          <Bode output={out} node={Math.min(node, labels.length - 1)} />
        </div>
      )}
    </section>
  )
}
