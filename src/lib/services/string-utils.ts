/**
 * Returns two segments of a string split by the last occurrence of a delimiter.
 * @param src Source string
 * @param delimiters An array of possible delimiters to slit on
 */
export function splitByLastDelimiter(
  src: string,
  delimiters: string[]
): [string | undefined, string] {
  let lastIndex = -1;
  let delimiterLength = 0;

  for (const delimiter of delimiters) {
    if (!delimiter) continue; // Skip empty string delimiters
    const index = src.lastIndexOf(delimiter);
    if (index > lastIndex) {
      lastIndex = index;
      delimiterLength = delimiter.length;
    }
  }

  if (lastIndex === -1) {
    return [undefined, src];
  }

  // Slice the string into two halves around the last delimiter
  const before = src.slice(0, lastIndex);
  const after = src.slice(lastIndex + delimiterLength);

  return [before, after];
}
