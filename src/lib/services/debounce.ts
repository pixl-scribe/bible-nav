/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Used to scale down high volume calls such as on key up or on scroll functions.
 */
export default function debounce<T extends (...args: any[]) => any>(
  func: T,
  wait: number
): (
  ...args: Parameters<T>
) => ReturnType<T> extends Promise<any>
  ? Promise<Awaited<ReturnType<T>>>
  : ReturnType<T> {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  return function (this: any, ...args: Parameters<T>) {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }

    return new Promise((resolve, reject) => {
      timeoutId = setTimeout(() => {
        try {
          const result = func.apply(this, args);
          resolve(result);
        } catch (error) {
          reject(error);
        }
      }, wait);
    }) as any;
  };
}
