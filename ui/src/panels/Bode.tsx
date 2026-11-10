import type { AcOutput } from '../sim/api'
import { formatSI } from './formatSI'

const W = 480, H = 150, L = 56, R = 10, T = 10, B = 24

/** Gain in dB; a zero magnitude (a node shorted to ground) is floored so the plot stays finite. */
export const toDb = (mag: number) => 20 * Math.log10(Math.max(mag, 1e-12))

/** toFixed without a minus sign on zero ("-0"). */
const tick = (v: number, digits: number) => v.toFixed(digits).replace(/^-(0(\.0+)?)$/, '$1')

function Plot({ freqs, ys, unit, label }: { freqs: number[]; ys: number[]; unit: string; label: string }) {
  const f0 = Math.log10(freqs[0])
  const f1 = Math.log10(freqs[freqs.length - 1])
  let lo = Math.min(...ys)
  let hi = Math.max(...ys)
  if (hi - lo < 1e-9) { lo -= 1; hi += 1 } // flat line: give it a visible range
  const x = (f: number) => L + ((Math.log10(f) - f0) / (f1 - f0 || 1)) * (W - L - R)
  const y = (v: number) => T + (1 - (v - lo) / (hi - lo)) * (H - T - B)
  const decades: number[] = []
  for (let d = Math.ceil(f0 - 1e-9); d <= f1 + 1e-9; d++) decades.push(10 ** d)
  return (
    <svg className="scope bode" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={label}>
      <rect x={L} y={T} width={W - L - R} height={H - T - B} className="scope-frame" />
      {[lo, (lo + hi) / 2, hi].map((v, i) => (
        <text key={i} x={L - 4} y={y(v)} textAnchor="end" dominantBaseline="middle" className="scope-tick">{tick(v, Math.abs(hi - lo) < 10 ? 1 : 0)} {unit}</text>
      ))}
      {decades.map((f) => (
        <g key={f}>
          <line x1={x(f)} x2={x(f)} y1={T} y2={H - B} className="bode-grid" />
          <text x={x(f)} y={H - 8} textAnchor={x(f) > W - R - 24 ? 'end' : 'middle'} className="scope-tick">{formatSI(f, 'Hz')}</text>
        </g>
      ))}
      <polyline points={freqs.map((f, i) => `${x(f)},${y(ys[i])}`).join(' ')} className="scope-line" fill="none" />
    </svg>
  )
}

/** Gain (dB) and phase (degrees) of one node against frequency, log frequency axis. */
/** `level`: the stimulus is not a single 1 V source, so the plot is output volts in dBV, not gain. */
export default function Bode({ output, node, kids = false, level = false }: { output: AcOutput; node: number; kids?: boolean; level?: boolean }) {
  if (output.freqs.length < 2) return null
  if (kids) return <Plot freqs={output.freqs} ys={output.magnitude.map((m) => (m[node] ?? 0) * 100)} unit="%" label="Loudness" /> // 100% = as loud as the wave going in
  const mag = output.magnitude.map((m) => toDb(m[node] ?? 0))
  const phase = output.phase_deg.map((p) => p[node] ?? 0)
  return (
    <div className="bode-plots">
      <Plot freqs={output.freqs} ys={mag} unit={level ? 'dBV' : 'dB'} label={level ? 'Level' : 'Gain'} />
      <Plot freqs={output.freqs} ys={phase} unit="°" label="Phase" />
    </div>
  )
}
