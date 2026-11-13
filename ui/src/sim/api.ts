import { invoke } from '@tauri-apps/api/core'

export type NodeId = number

// Mirrors nodal-core model.rs (serde tag = "kind", id flattened in).
export type PartJson = { id: string } & (
  | { kind: 'Resistor'; a: NodeId; b: NodeId; ohms: number; max_watts: number }
  | { kind: 'Capacitor'; a: NodeId; b: NodeId; farads: number }
  | { kind: 'Inductor'; a: NodeId; b: NodeId; henries: number }
  | { kind: 'VSource'; pos: NodeId; neg: NodeId; volts: number; wave?: { shape: 'Sine' | 'Square' | 'Triangle' | 'Sawtooth'; amp: number; freq: number; duty?: number; phase?: number } }
  | { kind: 'ISource'; from: NodeId; to: NodeId; amps: number }
  | { kind: 'Diode'; anode: NodeId; cathode: NodeId }
  | { kind: 'Led'; anode: NodeId; cathode: NodeId; max_amps: number }
  | { kind: 'Switch'; a: NodeId; b: NodeId; closed: boolean }
  | { kind: 'Mosfet'; d: NodeId; g: NodeId; s: NodeId; pmos: boolean; vth: number; kp: number; max_amps: number }
  | { kind: 'Opamp'; out: NodeId; plus: NodeId; minus: NodeId; gain: number; vsat: number; max_amps: number }
  | { kind: 'Bjt'; c: NodeId; b: NodeId; e: NodeId; pnp: boolean; beta: number; max_amps: number }
)

/** AC sweep amplitude and phase text per voltage source id; saved beside the parts, ignored by the core. */
export type AcSetup = Record<string, { amp?: string; phase?: string }>

/** Keeps only string amp/phase entries of a loaded file's `ac`; anything else in an edited file is dropped. */
export function cleanAc(raw: unknown): AcSetup {
  const out: AcSetup = {}
  if (raw && typeof raw === 'object') {
    for (const [id, v] of Object.entries(raw)) {
      const { amp, phase } = (v && typeof v === 'object' ? v : {}) as Record<string, unknown>
      out[id] = { ...(typeof amp === 'string' && { amp }), ...(typeof phase === 'string' && { phase }) }
    }
  }
  return out
}

export interface CircuitJson {
  version: 1
  parts: PartJson[]
  ac?: AcSetup
}

export interface DamageWarning {
  part_id: string
  kind: 'OverPower' | 'OverCurrent'
  peak: number
  limit: number
}

export interface SimOutput {
  times: number[]
  node_voltages: number[][]
  part_currents: Record<string, number[]>
  warnings: DamageWarning[]
}

export const runSimulation = (c: CircuitJson, stop: number, step: number): Promise<SimOutput> =>
  invoke<SimOutput>('run_simulation', { circuitJson: JSON.stringify(c), stop, step })

export interface AcOutput {
  freqs: number[]
  /** [freq][node], volts for the stimuli sent (gain for a 1 V stimulus) */
  magnitude: number[][]
  /** [freq][node], degrees */
  phase_deg: number[][]
}

/** One voltage source driven at AC: peak volts and phase in degrees. */
export interface AcStimulus { id: string; amp: number; phase_deg: number }

export const runAc = (c: CircuitJson, stimuli: AcStimulus[], fStart: number, fStop: number, pointsPerDecade: number): Promise<AcOutput> =>
  invoke<AcOutput>('run_ac', { circuitJson: JSON.stringify(c), stimuli, fStart, fStop, pointsPerDecade })

export const saveCircuit = (path: string, c: CircuitJson): Promise<void> =>
  invoke<void>('save_circuit', { path, circuitJson: JSON.stringify(c) })

export const loadCircuit = async (path: string): Promise<CircuitJson> =>
  JSON.parse(await invoke<string>('load_circuit', { path })) as CircuitJson
