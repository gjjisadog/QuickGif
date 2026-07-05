type WheelSecondsInput = {
  deltaX: number
  deltaY: number
  trackPixels: number
  visibleSeconds: number
}

type MoveFocusRangeInput = {
  start: number
  end: number
  duration: number
  deltaSeconds: number
}

type ZoomFocusRangeInput = {
  start: number
  end: number
  duration: number
  anchor: number
  deltaY: number
  minSpan: number
}

type WheelStepInput = {
  accumulator: number
  delta: number
  threshold: number
}

export function wheelSecondsFromDelta(input: WheelSecondsInput) {
  if (input.trackPixels <= 0 || input.visibleSeconds <= 0) {
    return 0
  }

  if (Math.abs(input.deltaX) < Math.abs(input.deltaY)) {
    return 0
  }

  const dampedDelta = clamp(input.deltaX, -120, 120)
  return roundToMillis((dampedDelta / input.trackPixels) * input.visibleSeconds)
}

export function moveFocusRange(input: MoveFocusRangeInput): [number, number] {
  const span = Math.max(0, input.end - input.start)
  if (input.duration <= 0 || span <= 0) {
    return [0, 0]
  }

  const nextStart = clamp(input.start + input.deltaSeconds, 0, Math.max(0, input.duration - span))
  return [roundToMillis(nextStart), roundToMillis(nextStart + span)]
}

export function zoomFocusRange(input: ZoomFocusRangeInput): [number, number] {
  if (input.duration <= 0) {
    return [0, 0]
  }

  const currentSpan = clamp(input.end - input.start, input.minSpan, input.duration)
  const zoomFactor = input.deltaY < 0 ? 0.82 : 1.18
  const nextSpan = clamp(currentSpan * zoomFactor, input.minSpan, input.duration)
  const anchor = clamp(input.anchor, 0, input.duration)
  const anchorRatio = currentSpan <= 0 ? 0.5 : clamp((anchor - input.start) / currentSpan, 0, 1)
  const nextStart = clamp(anchor - nextSpan * anchorRatio, 0, Math.max(0, input.duration - nextSpan))

  return [roundToMillis(nextStart), roundToMillis(nextStart + nextSpan)]
}

export function consumeWheelStep(input: WheelStepInput) {
  const nextAccumulator = input.accumulator + input.delta
  if (Math.abs(nextAccumulator) < input.threshold) {
    return {
      steps: 0,
      accumulator: nextAccumulator,
    }
  }

  return {
    steps: Math.sign(nextAccumulator),
    accumulator: 0,
  }
}

function roundToMillis(value: number) {
  return Math.round(value * 1000) / 1000
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}
