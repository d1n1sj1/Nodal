const PREFIXES = ['p', 'n', 'µ', 'm', '', 'k', 'M', 'G']

/** 3 significant digits with an SI prefix: formatSI(0.0018,'A') === '1.80 mA'. */
export function formatSI(value: number, unit: string): string {
  if (!Number.isFinite(value)) return '—'
  if (value === 0) return `0.00 ${unit}`
  const idx = (v: number) => Math.min(7, Math.max(0, Math.floor(Math.log10(Math.abs(v)) / 3) + 4))
  const fmt = (i: number) => {
    const r = Number((value / 10 ** ((i - 4) * 3)).toPrecision(3)) // round to 3 sig digits first
    const mag = Math.abs(r) < 1 ? 0 : Math.floor(Math.log10(Math.abs(r)))
    return r.toFixed(Math.max(0, 2 - mag)).replace(/^-(?=0(\.0+)?$)/, '') // no "-0.00"
  }
  let i = idx(value)
  let out = fmt(i)
  if (Math.abs(Number(out)) >= 1000 && i < 7) out = fmt(++i) // 999.6 rounds up to 1.00 k
  return `${out} ${PREFIXES[i]}${unit}`
}

/** Parse "10m", "5u", "1k", "2.5" (plain or SI suffix); NaN if invalid or non-finite.
 *  Suffixes are case-sensitive: "m" = milli, "M" = mega. */
export function parseSI(text: string): number {
  const m = /^\s*([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)\s*([pnuµmkMG]?)\s*$/.exec(text)
  if (!m) return NaN
  const p = m[2] === '' ? '' : m[2] === 'u' ? 'µ' : m[2]
  const i = PREFIXES.indexOf(p)
  const e = (i - 4) * 3
  const v = e < 0 ? Number(m[1]) / 10 ** -e : Number(m[1]) * 10 ** e // divide: 10*1e-6 is inexact
  return Number.isFinite(v) ? v : NaN
}
