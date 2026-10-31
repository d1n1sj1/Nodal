import { useState } from 'react'
import type { SimOutput } from '../sim/api'
import { formatSI } from './formatSI'

const W = 480, H = 200, L = 56, R = 10, T = 10, B = 26

interface Props { output: SimOutput | null; selectedNode: number; onCursor?: (t: number) => void }

export default function Scope({ output, selectedNode, onCursor }: Props) {
  const [cur, setCur] = useState<number | null>(null)
  const times = output?.times ?? []
  const ys = times.map((_, i) => output!.node_voltages[i]?.[selectedNode] ?? 0)
  const n = Math.min(times.length, ys.length)
  const t0 = n ? times[0] : 0
  let t1 = n ? times[n - 1] : 1
  if (t1 <= t0) t1 = t0 + 1
  let lo = n ? Math.min(...ys) : 0
  let hi = n ? Math.max(...ys) : 1
  if (hi - lo < 1e-12) { lo -= 0.5; hi += 0.5 } // flat line: give it a visible range
  const x = (t: number) => L + ((t - t0) / (t1 - t0)) * (W - L - R)
  const y = (v: number) => T + (1 - (v - lo) / (hi - lo)) * (H - T - B)
  const pts = times.slice(0, n).map((t, i) => `${x(t)},${y(ys[i])}`).join(' ')
  const yTicks = [lo, (lo + hi) / 2, hi]
  const xTicks = [t0, (t0 + t1) / 2, t1]

  const move = (e: React.PointerEvent<SVGSVGElement>) => {
    const r = e.currentTarget.getBoundingClientRect()
    const px = ((e.clientX - r.left) / (r.width || W)) * W
    const t = Math.min(t1, Math.max(t0, t0 + ((px - L) / (W - L - R)) * (t1 - t0)))
    setCur(t)
    onCursor?.(t)
  }

  return (
    <svg className="scope" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Scope" onPointerMove={move}>
      <rect x={L} y={T} width={W - L - R} height={H - T - B} className="scope-frame" />
      {yTicks.map((v, i) => (
        <text key={`y${i}`} x={L - 4} y={y(v)} textAnchor="end" dominantBaseline="middle" className="scope-tick">{formatSI(v, 'V')}</text>
      ))}
      {xTicks.map((t, i) => (
        <text key={`x${i}`} x={x(t)} y={H - 8} textAnchor={i === 0 ? 'start' : i === 2 ? 'end' : 'middle'} className="scope-tick">{formatSI(t, 's')}</text>
      ))}
      {n > 0 && <polyline points={pts} className="scope-line" fill="none" />}
      {n > 0 && cur !== null && <line x1={x(cur)} x2={x(cur)} y1={T} y2={H - B} className="scope-cursor" />}
    </svg>
  )
}
