import type { Schematic } from '../editor/schematic'
import type { Mode } from '../modes/mode'
import type { DamageWarning } from '../sim/api'
import { partNameFor, warningText } from './warningText'

interface Props { warnings?: DamageWarning[]; mode: Mode; schematic?: Schematic }

export default function WarningBanner({ warnings, mode, schematic }: Props) {
  if (!warnings?.length) return null
  return (
    <div className="warning-banner" role="alert">
      <ul>
        {warnings.map((w, i) => (
          <li key={i}>{warningText(w, mode, schematic ? partNameFor(schematic, w.part_id) : w.part_id)}</li>
        ))}
      </ul>
    </div>
  )
}
