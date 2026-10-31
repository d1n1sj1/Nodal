import type { Schematic } from '../editor/schematic'
import type { Mode } from '../modes/mode'
import type { DamageWarning } from '../sim/api'
import { formatSI } from './formatSI'

/** Placed-part id for a core id: kids parts expand to `pN-resistor` / `pN-inductor`; an exact placed id wins (reloaded files). */
const placedId = (s: Schematic, coreId: string) =>
  s.parts.some((p) => p.id === coreId) ? coreId : coreId.replace(/-(resistor|inductor)$/, '')

export const partNameFor = (s: Schematic, coreId: string): string =>
  s.parts.find((p) => p.id === placedId(s, coreId))?.def.name ?? coreId

export const damagedIds = (warnings: DamageWarning[] | undefined, s: Schematic): Set<string> =>
  new Set((warnings ?? []).map((w) => placedId(s, w.part_id)))

// formatSI pads to 3 sig digits ("47.0 mA"); trim for readable messages.
const num = (v: number, unit: string) => formatSI(v, unit).replace(/(\.\d*?)0+ /, '$1 ').replace(/\. /, ' ')

export function warningText(w: DamageWarning, mode: Mode, partName: string): string {
  if (mode === 'kids') {
    return w.kind === 'OverCurrent'
      ? `Careful! The ${partName} could burn out — too much electricity is flowing through it. Try adding a resistor.`
      : `Careful! The ${partName} is getting too hot and could burn out. Try adding a resistor.`
  }
  const unit = w.kind === 'OverPower' ? 'W' : 'A'
  const what = w.kind === 'OverPower' ? 'power' : 'current'
  const peak = w.peak >= 1e300 ? `far above the ${num(w.limit, unit)} limit` : `${num(w.peak, unit)} exceeds the ${num(w.limit, unit)} limit`
  return `${partName} (${w.part_id}): peak ${what} ${peak}`
}
