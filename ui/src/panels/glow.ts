import type { Schematic } from '../editor/schematic'
import type { SimOutput } from '../sim/api'

/** Brightness 0..1 per placed resistor-kind part: final-sample power over its watt rating. Only parts with a rating can glow. */
export function glowLevels(out: SimOutput | null | undefined, s: Schematic): Map<string, number> {
  const m = new Map<string, number>()
  if (!out) return m
  for (const p of s.parts) {
    const i = out.part_currents[p.id]?.at(-1)
    const { ohms, max_watts } = p.values
    if (p.def.kind !== 'Resistor' || i === undefined || !(max_watts > 0)) continue
    m.set(p.id, Math.min(1, (i * i * ohms) / max_watts))
  }
  return m
}
