import { useState } from 'react'
import type { PlacedPart } from '../editor/schematic'
import type { Mode } from '../modes/mode'
import { tileFor } from '../parts/tiles'
import { formatSI, parseSI } from './formatSI'

const LABELS: Record<string, string> = {
  ohms: 'Resistance (Ω)', max_watts: 'Max power (W)', farads: 'Capacitance (F)', henries: 'Inductance (H)',
  volts: 'Voltage (V)', amps: 'Current (A)', max_amps: 'Max current (A)',
}
const KIDS_LABELS: Record<string, string> = {
  ohms: 'resistance (ohms)', max_watts: 'power limit (watts)', farads: 'farads', henries: 'henries',
  volts: 'volts', amps: 'amps', max_amps: 'current limit (amps)',
}
const SIGNED = ['volts', 'amps'] // sources may be negative or zero; every other value must be > 0
const valid = (k: string, v: number) => Number.isFinite(v) && (SIGNED.includes(k) || v > 0)

/** Exact text for a value: SI form when it parses back to the same number, else plain. */
const text = (v: number) => { const f = formatSI(v, '').trim(); return parseSI(f) === v ? f : String(v) }

interface Props { part: PlacedPart; mode: Mode; onChange: (values: Record<string, number>) => void }

/** Value editor for the selected part. Mount with key={part.id} so drafts reset per part. */
export default function Properties({ part, mode, onChange }: Props) {
  const kids = mode === 'kids'
  const [drafts, setDrafts] = useState<Record<string, string>>({})
  const name = kids ? tileFor(part.def).name : part.def.name
  // multi-part kids items (motor) build from fixed defaults, so there is nothing to edit
  const keys = part.def.builds && part.def.builds.length > 1 ? [] : Object.keys(part.def.defaults).filter((k) => k !== 'closed')
  const isSwitch = part.def.kind === 'Switch'
  if (!keys.length && !isSwitch) return null

  const edit = (k: string, t: string) => {
    setDrafts({ ...drafts, [k]: t })
    const v = parseSI(t)
    if (valid(k, v)) onChange({ [k]: v })
  }
  const bad = (k: string) => {
    const t = drafts[k]
    if (t === undefined || valid(k, parseSI(t))) return null
    if (kids) return SIGNED.includes(k) ? 'Try a number, like 9.' : 'Try a number bigger than zero, like 9.'
    return SIGNED.includes(k) ? `${LABELS[k] ?? k}: enter a number like 5, -2.5 or 500m.` : `${LABELS[k] ?? k}: enter a number above zero, like 1k or 4.7u.`
  }
  const closed = part.values.closed !== 0

  return (
    <section className="properties" aria-label={`${name} properties`}>
      <h2>{kids ? name : `${name} ${part.id}`}</h2>
      {keys.map((k) => {
        const label = kids ? `${name} ${KIDS_LABELS[k] ?? k}` : LABELS[k] ?? k
        const err = bad(k)
        return (
          <div key={k} className="prop-row">
            <label>
              {label}{' '}
              <input value={drafts[k] ?? text(part.values[k])} onChange={(e) => edit(k, e.target.value)} size={8}
                aria-invalid={err ? true : undefined} aria-describedby={err ? `prop-err-${k}` : undefined} />
            </label>
            {err && <span id={`prop-err-${k}`} className="prop-error" role="alert">{err}</span>}
          </div>
        )
      })}
      {isSwitch && (
        <div className="mode-switch" role="group" aria-label={`${name} position`}>
          <button type="button" aria-pressed={!closed} onClick={() => onChange({ closed: 0 })}>Open</button>
          <button type="button" aria-pressed={closed} onClick={() => onChange({ closed: 1 })}>Closed</button>
        </div>
      )}
    </section>
  )
}
