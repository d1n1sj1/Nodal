import { useState } from 'react'
import type { Mode } from '../modes/mode'
import type { SimOutput } from '../sim/api'
import { formatSI } from './formatSI'

export { formatSI }

interface Props { output: SimOutput | null; cursorTime: number | null; selectedNode: number; mode?: Mode }

export default function Readouts({ output, cursorTime, selectedNode, mode = 'engineer' }: Props) {
  const ids = Object.keys(output?.part_currents ?? {})
  const [part, setPart] = useState<string>('')
  const partId = ids.includes(part) ? part : ids[0]
  const times = output?.times ?? []
  let k = 0 // nearest sample to the cursor (last sample when no cursor)
  if (cursorTime !== null && times.length) {
    k = times.reduce((b, t, i) => (Math.abs(t - cursorTime) < Math.abs(times[b] - cursorTime) ? i : b), 0)
  } else k = times.length - 1
  const has = k >= 0 && k < times.length
  const v = has ? output!.node_voltages[k]?.[selectedNode] : NaN
  const i = has && partId ? output!.part_currents[partId]?.[k] : NaN
  const t = has ? times[k] : NaN

  if (mode === 'kids') {
    return (
      <div className="readouts readouts-kids">
        <div>Voltage <b>{formatSI(v, 'V')}</b></div>
        <div>Time <b>{formatSI(t, 's')}</b></div>
      </div>
    )
  }
  return (
    <div className="readouts">
      <div>V (node {selectedNode}) <b>{formatSI(v, 'V')}</b></div>
      <div>
        I{' '}
        <select aria-label="Part" value={partId ?? ''} onChange={(e) => setPart(e.target.value)}>
          {ids.map((id) => <option key={id} value={id}>{id}</option>)}
        </select>{' '}
        <b>{formatSI(i, 'A')}</b>
      </div>
      <div>t <b>{formatSI(t, 's')}</b></div>
    </div>
  )
}
