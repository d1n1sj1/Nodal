import { useMemo, useState } from 'react'
import type { SimOutput } from '../sim/api'
import { formatSI } from './formatSI'

const MAX_DRAWN = 2000 // points per line; beyond this the line looks the same
const W = 480, H = 200, L = 56, R = 10, T = 10, B = 26

interface Props { output: SimOutput | null; selectedNode: number; onCursor?: (t: number) => void }

export default function Scope({ output, selectedNode, onCursor }: Props) {
  const [cur, setCur] = useState<number | null>(null)
  // Derived once per run and node: a mouse move only re-renders the cursor, not a million points.
  const { n, t0, t1, lo, hi, pts } = useMemo(() => {
    const times = output?.times ?? []
    const ys = times.map((_, i) => output!.node_voltages[i]?.[selectedNode] ?? 0)
    const n = Math.min(times.length, ys.length)
    const t0 = n ? times[0] : 0
    let t1 = n ? times[n - 1] : 1
    if (t1 <= t0) t1 = t0 + 1
    let lo = n ? Infinity : 0 // loop, not Math.min(...ys): spread overflows the stack on huge arrays
    let hi = n ? -Infinity : 1
    for (let i = 0; i < n; i++) { if (ys[i] < lo) lo = ys[i]; if (ys[i] > hi) hi = ys[i] }
    if (hi - lo < 1e-12) { lo -= 0.5; hi += 0.5 } // flat line: give it a visible range
    const X = (t: number) => L + ((t - t0) / (t1 - t0)) * (W - L - R)
    const Y = (v: number) => T + (1 - (v - lo) / (hi - lo)) * (H - T - B)
    // More samples than pixels: keep each bucket's lowest and highest sample, so peaks survive.
    const keep: number[] = []
    if (n <= MAX_DRAWN) for (let i = 0; i < n; i++) keep.push(i)
    else for (let k = 0; k < MAX_DRAWN / 2; k++) {
      const a = Math.floor((k * n) / (MAX_DRAWN / 2)), z = Math.floor(((k + 1) * n) / (MAX_DRAWN / 2))
      let imin = a, imax = a
      for (let i = a; i < z; i++) { if (ys[i] < ys[imin]) imin = i; if (ys[i] > ys[imax]) imax = i }
      keep.push(...(imin <= imax ? [imin, imax] : [imax, imin]))
    }
    return { n, t0, t1, lo, hi, pts: keep.map((i) => `${X(times[i])},${Y(ys[i])}`).join(' ') }
  }, [output, selectedNode])
  const x = (t: number) => L + ((t - t0) / (t1 - t0)) * (W - L - R)
  const y = (v: number) => T + (1 - (v - lo) / (hi - lo)) * (H - T - B)
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
