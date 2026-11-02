import { invoke } from '@tauri-apps/api/core'

export type NodeId = number

// Mirrors nodal-core model.rs (serde tag = "kind", id flattened in).
export type PartJson = { id: string } & (
  | { kind: 'Resistor'; a: NodeId; b: NodeId; ohms: number; max_watts: number }
  | { kind: 'Capacitor'; a: NodeId; b: NodeId; farads: number }
  | { kind: 'Inductor'; a: NodeId; b: NodeId; henries: number }
  | { kind: 'VSource'; pos: NodeId; neg: NodeId; volts: number }
  | { kind: 'ISource'; from: NodeId; to: NodeId; amps: number }
  | { kind: 'Diode'; anode: NodeId; cathode: NodeId }
  | { kind: 'Led'; anode: NodeId; cathode: NodeId; max_amps: number }
  | { kind: 'Switch'; a: NodeId; b: NodeId; closed: boolean }
  | { kind: 'Mosfet'; d: NodeId; g: NodeId; s: NodeId; pmos: boolean; vth: number; kp: number; max_amps: number }
  | { kind: 'Bjt'; c: NodeId; b: NodeId; e: NodeId; pnp: boolean; beta: number; max_amps: number }
)

export interface CircuitJson {
  version: 1
  parts: PartJson[]
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

export const saveCircuit = (path: string, c: CircuitJson): Promise<void> =>
  invoke<void>('save_circuit', { path, circuitJson: JSON.stringify(c) })

export const loadCircuit = async (path: string): Promise<CircuitJson> =>
  JSON.parse(await invoke<string>('load_circuit', { path })) as CircuitJson
