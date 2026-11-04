import { useState } from 'react'
import { buildCircuit, nodeLabels } from '../editor/buildCircuit'
import type { Schematic } from '../editor/schematic'
import { runAc, type AcOutput } from '../sim/api'
import Bode from './Bode'
import type { Mode } from '../modes/mode'
import { parseSI } from './formatSI'
import { kidsError } from './kidsError'

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

/** Small-signal frequency sweep around the DC operating point. One voltage source is the 1 V AC stimulus. */
export default function AcPanel({ schematic, mode = 'engineer' }: { schematic: Schematic; mode?: Mode }) {
  const kids = mode === 'kids'
  const [source, setSource] = useState('')
  const [start, setStart] = useState('10')
  const [stop, setStop] = useState('100k')
  const [ppd, setPpd] = useState('20')
  const [node, setNode] = useState<number | null>(null)
  const [ran, setRan] = useState<{ out: AcOutput; of: Schematic } | null>(null)
  const out = ran && ran.of === schematic ? ran.out : null // any edit makes the plot stale
  const [error, setError] = useState<string | null>(null)
  const [running, setRunning] = useState(false)

  const sources = schematic.parts.filter((p) => p.def.kind === 'VSource')
  const chosen = sources.some((p) => p.id === source) ? source : (kids ? sources.find((p) => 'amp' in p.def.defaults) : undefined)?.id ?? sources[0]?.id // kids sweep the Wave tile if there is one
  const labels = nodeLabels(schematic)
  const shown = Math.min(node ?? (kids ? labels.length - 1 : 1), labels.length - 1) // kids start on the last node: node 1 is the wave itself, always 100%

  const run = async () => {
    const [f0, f1, n] = [parseSI(start), parseSI(stop), parseSI(ppd)]
    if (!chosen) return setError(kids ? 'Add a Wave or a Battery first: it is the sound the sweep sends in.' : 'Add a voltage source first: it is the signal the sweep sends in.')
    if (!(f0 > 0) || !(f1 > f0)) return setError('Enter frequencies like 10 and 100k, with start below stop.')
    if (!(n >= 1) || !Number.isInteger(n)) return setError('Points per decade must be a whole number, like 20.')
    setError(null)
    setRunning(true)
    try {
      setRan({ out: await runAc(buildCircuit(schematic), chosen, f0, f1, n), of: schematic })
    } catch (e) {
      setRan(null)
      setError(kids ? kidsError(message(e)) : message(e))
    } finally {
      setRunning(false)
    }
  }

  return (
    <section className="acpanel" aria-label="AC sweep">
      <div className="runbar">
        <strong>{kids ? 'Pitch sweep' : 'AC sweep'}</strong>
        {!kids && <><label>Source{' '}
          <select aria-label="AC source" value={chosen ?? ''} onChange={(e) => setSource(e.target.value)}>
            {sources.map((p) => <option key={p.id} value={p.id}>{p.id}</option>)}
          </select>
        </label>
        <label>From <input aria-label="Start frequency" value={start} onChange={(e) => setStart(e.target.value)} size={6} /></label>
        <label>To <input aria-label="Stop frequency" value={stop} onChange={(e) => setStop(e.target.value)} size={6} /></label>
        <label>Points/decade <input aria-label="Points per decade" value={ppd} onChange={(e) => setPpd(e.target.value)} size={4} /></label></>}
        <button type="button" className="run-btn" disabled={running} onClick={run}>{kids ? 'Sweep pitches' : 'Sweep'}</button>
        {error && <span className="runbar-error" role="alert">{error}</span>}
      </div>
      {out && (
        <div className="results">
          <label>Node{' '}
            <select aria-label="AC node" value={shown} onChange={(e) => setNode(Number(e.target.value))}>
              {labels.map((l, i) => <option key={i} value={i}>{l}</option>)}
            </select>
          </label>
          <Bode output={out} node={shown} kids={kids} />
        </div>
      )}
    </section>
  )
}
