import assert from 'node:assert/strict'
import { readFile, writeFile, rm } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import ts from 'typescript'

const sourcePath = new URL('../src/trackpadInteraction.ts', import.meta.url)
const source = await readFile(sourcePath, 'utf8')
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ES2022,
    target: ts.ScriptTarget.ES2022,
    verbatimModuleSyntax: true,
  },
})

const tempFile = join(tmpdir(), `quickgif-trackpad-${Date.now()}.mjs`)
await writeFile(tempFile, compiled.outputText)
const mod = await import(`file://${tempFile}`)
await rm(tempFile, { force: true })

assert.equal(
  mod.wheelSecondsFromDelta({ deltaX: 120, deltaY: 4, trackPixels: 600, visibleSeconds: 12 }),
  2.4,
)

assert.equal(
  mod.wheelSecondsFromDelta({ deltaX: 8, deltaY: 80, trackPixels: 600, visibleSeconds: 12 }),
  0,
)

assert.deepEqual(mod.moveFocusRange({ start: 10, end: 22, duration: 100, deltaSeconds: 5 }), [15, 27])
assert.deepEqual(mod.moveFocusRange({ start: 10, end: 22, duration: 100, deltaSeconds: -30 }), [0, 12])

const zoomedIn = mod.zoomFocusRange({
  start: 10,
  end: 22,
  duration: 100,
  anchor: 16,
  deltaY: -30,
  minSpan: 1,
})
assert.equal(Number((zoomedIn[1] - zoomedIn[0]).toFixed(2)) < 12, true)
assert.equal(zoomedIn[0] < 16 && zoomedIn[1] > 16, true)

assert.deepEqual(mod.consumeWheelStep({ accumulator: 60, delta: 25, threshold: 80 }), {
  steps: 1,
  accumulator: 0,
})
assert.deepEqual(mod.consumeWheelStep({ accumulator: -70, delta: -20, threshold: 80 }), {
  steps: -1,
  accumulator: 0,
})
