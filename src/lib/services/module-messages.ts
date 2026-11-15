/**
 * Turns problems and errors from Rust into text in the reader's language.
 * Rust never sends sentences: a problem is a message key with values, and
 * an error is a code. The English technical detail, if any, is kept apart
 * so the page can show it smaller.
 */

/** A problem from Rust. Matches `Problem` in src-tauri/src/modules/problem.rs. */
export interface Problem {
  key: string;
  values: Record<string, string>;
  detail?: string;
}

/** An error from a Rust command. Matches `ModuleError`'s serialized form. */
export interface CommandError {
  code: string;
  detail: string;
}

/** svelte-i18n's `$_`. */
export type Translate = (
  key: string,
  options?: { values?: Record<string, string | number> }
) => string;

/** A message for the reader, and the technical detail beneath it. */
export interface Message {
  text: string;
  detail?: string;
}

export function problemMessage(problem: Problem, t: Translate): Message {
  return {
    text: t(problem.key, { values: problem.values }),
    detail: problem.detail,
  };
}

function isCommandError(value: unknown): value is CommandError {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as CommandError).code === 'string'
  );
}

/** Whether an error is the reader cancelling, which needs no message. */
export function isCancelled(error: unknown): boolean {
  return isCommandError(error) && error.code === 'cancelled';
}

/**
 * The message for anything a command threw: a Rust error by its code, or,
 * for anything else (such as a failed `invoke`), a general message with the
 * error as detail.
 */
export function errorMessage(error: unknown, t: Translate): Message {
  if (isCommandError(error)) {
    return {
      text: t(`error.${error.code}`),
      detail: error.detail || undefined,
    };
  }
  return { text: t('error.task'), detail: String(error) };
}
