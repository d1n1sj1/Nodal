import { describe, expect, it, vi } from 'vitest'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

import { loadCircuit, runSimulation, saveCircuit, type CircuitJson } from './api'

const c: CircuitJson = { version: 1, parts: [{ id: 'V1', kind: 'VSource', pos: 1, neg: 0, volts: 5 }] }

describe('api', () => {
  it('runSimulation_passes_args_to_invoke', async () => {
    const out = { times: [0], node_voltages: [[0]], part_currents: {}, warnings: [] }
    invoke.mockResolvedValue(out)
    expect(await runSimulation(c, 1, 0.1)).toBe(out)
    expect(invoke).toHaveBeenCalledWith('run_simulation', { circuitJson: JSON.stringify(c), stop: 1, step: 0.1 })
  })

  it('saveCircuit passes path and serialised circuit', async () => {
    invoke.mockResolvedValue(undefined)
    await saveCircuit('/x.json', c)
    expect(invoke).toHaveBeenLastCalledWith('save_circuit', { path: '/x.json', circuitJson: JSON.stringify(c) })
  })

  it('loadCircuit parses returned json', async () => {
    invoke.mockResolvedValue(JSON.stringify(c))
    expect(await loadCircuit('/x.json')).toEqual(c)
    expect(invoke).toHaveBeenLastCalledWith('load_circuit', { path: '/x.json' })
  })
})
