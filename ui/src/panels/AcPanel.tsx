import { useState } from 'react'
import { buildCircuit, nodeLabels } from '../editor/buildCircuit'
import type { Schematic } from '../editor/schematic'
import { runAc, type AcOutput, type AcStimulus } from '../sim/api'
import Bode from './Bode'
import type { Mode } from '../modes/mode'
import { parseSI } from './formatSI'
import { kidsError } from './kidsError'

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

/** Small-signal frequency sweep around the DC operating point. Each voltage source can be driven with an amplitude and a phase; the plot is their sum. */
export default function AcPanel({ schematic, mode = 'engineer' }: { schematic: Schematic; mode?: Mode }) {
  const kids = mode === 'kids'
  const [amps, setAmps] = useState<Record<string, string>>({})
  const [phases, setPhases] = useState<Record<string, string>>({})
  const [start, setStart] = useState('10')
  const [stop, setStop] = useState('100k')
  const [ppd, setPpd] = useState('20')
  const [node, setNode] = useState<number | null>(null)
  const [ran, setRan] = useState<{ out: AcOutput; of: Schematic; level: boolean } | null>(null)
  const out = ran && ran.of === schematic ? ran.out : null // any edit makes the plot stale
  const [error, setError] = useState<string | null>(null)
  const [running, setRunning] = useState(false)

  const sources = schematic.parts.filter((p) => p.def.kind === 'VSource')
  const chosen = (kids ? sources.find((p) => 'shape' in p.def.defaults) : undefined)?.id ?? sources[0]?.id // kids sweep the Wave tile if there is one
  const ampText = (id: string) => amps[id] ?? (id === sources[0]?.id ? '1' : '0') // the first source is the 1 V stimulus until edited
  const labels = nodeLabels(schematic)
  const shown = Math.min(node ?? (kids ? labels.length - 1 : 1), labels.length - 1) // kids start on the last node: node 1 is the wave itself, always 100%

  const run = async () => {
    const [f0, f1, n] = [parseSI(start), parseSI(stop), parseSI(ppd)]
    if (!chosen) return setError(kids ? 'Add a Wave or a Battery first: it is the sound the sweep sends in.' : 'Add a voltage source first: it is the signal the sweep sends in.')
    const stimuli: AcStimulus[] = kids ? [{ id: chosen, amp: 1, phase_deg: 0 }] : sources.map((p) => ({ id: p.id, amp: parseSI(ampText(p.id)), phase_deg: parseSI(phases[p.id] ?? '0') }))
    if (stimuli.some((s) => !Number.isFinite(s.amp) || !Number.isFinite(s.phase_deg))) return setError('Amplitude and phase must be numbers, like 1 or 90.')
    const active = stimuli.filter((s) => s.amp !== 0)
    if (!active.length) return setError('Give at least one source an amplitude other than zero.')
    if (!(f0 > 0) || !(f1 > f0)) return setError('Enter frequencies like 10 and 100k, with start below stop.')
    if (!(n >= 1) || !Number.isInteger(n)) return setError('Points per decade must be a whole number, like 20.')
    setError(null)
    setRunning(true)
    try {
      setRan({ out: await runAc(buildCircuit(schematic), active, f0, f1, n), of: schematic, level: !(active.length === 1 && active[0].amp === 1) })
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
        {!kids && <>{sources.map((p) => (
          <span key={p.id} className="ac-source">
            <label>{p.id} amplitude (V) <input aria-label={`${p.id} AC amplitude`} value={ampText(p.id)} onChange={(e) => setAmps({ ...amps, [p.id]: e.target.value })} size={4} /></label>
            <label>phase (°) <input aria-label={`${p.id} AC phase`} value={phases[p.id] ?? '0'} onChange={(e) => setPhases({ ...phases, [p.id]: e.target.value })} size={4} /></label>
          </span>
        ))}
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
          <Bode output={out} node={shown} kids={kids} level={ran?.level} />
        </div>
      )}
    </section>
  )
}
