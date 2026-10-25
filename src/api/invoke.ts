import { Channel, invoke } from '@tauri-apps/api/core';

import type { Progress } from './types';

/** What every command rejects with: `{ code, message }` from the Rust side. */
export class AppError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'AppError';
  }
}

function toAppError(error: unknown): AppError {
  if (error instanceof AppError) return error;
  if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) {
    return new AppError(String(error.code), String(error.message));
  }
  return new AppError('unknown', String(error));
}

export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}

/** Calls a long-running command that streams `{ phase, done, total }`. */
export function callWithProgress<T>(
  command: string,
  args: Record<string, unknown>,
  onProgress: (progress: Progress) => void,
): Promise<T> {
  const channel = new Channel<Progress>();
  channel.onmessage = onProgress;
  return call<T>(command, { ...args, onProgress: channel });
}

export function errorMessage(error: unknown): string {
  return toAppError(error).message;
}

export function errorCode(error: unknown): string {
  return toAppError(error).code;
}
