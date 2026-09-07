export function calcLimitAndOffsetWithButter(
  buffer: number,
  index: number
): {
  limit: number;
  offset: number;
} {
  const priorBufferCount = Math.min(buffer, index);
  const limit = priorBufferCount + buffer + 1;
  const offset = Math.max(index - buffer, 0);
  return { limit, offset };
}
