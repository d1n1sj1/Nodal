const PREFIXES = ['p', 'n', 'µ', 'm', '', 'k', 'M', 'G']

/** 3 significant digits with an SI prefix: formatSI(0.0018,'A') === '1.80 mA'. */
export function formatSI(value: number, unit: string): string {
  if (!Number.isFinite(value)) return '—'
  if (value === 0) return `0.00 ${unit}`
  const idx = (v: number) => Math.min(7, Math.max(0, Math.floor(Math.log10(Math.abs(v)) / 3) + 4))
  const fmt = (i: number) => {
    const s = value / 10 ** ((i - 4) * 3)
    const mag = Math.abs(s) < 1 ? 0 : Math.floor(Math.log10(Math.abs(s)))
    return s.toFixed(Math.max(0, 2 - mag))
  }
  let i = idx(value)
  let out = fmt(i)
  if (Math.abs(Number(out)) >= 1000 && i < 7) out = fmt(++i) // 999.6 rounds up to 1.00 k
  return `${out} ${PREFIXES[i]}${unit}`
}

/** Parse "10m", "5u", "1k", "2.5" (plain or SI suffix); NaN if invalid. */
export function parseSI(text: string): number {
  const m = /^\s*([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)\s*([pnuµmkMG]?)\s*$/.exec(text)
  if (!m) return NaN
  const p = m[2] === '' ? '' : m[2] === 'u' ? 'µ' : m[2]
  const i = PREFIXES.indexOf(p)
  const e = (i - 4) * 3
  return e < 0 ? Number(m[1]) / 10 ** -e : Number(m[1]) * 10 ** e // divide: 10*1e-6 is inexact
}
