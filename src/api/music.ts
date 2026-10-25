import { call, callWithProgress } from './invoke';
import type { CopyView, PlanView, Progress } from './types';

export interface TransferRequest {
  sources: string[];
  destDir: string;
  /** Mirror: remove card files the (single) source folder no longer has. */
  deleteExtra: boolean;
}

export const planTransfer = (request: TransferRequest) =>
  call<PlanView>('music_plan', { ...request });

export const copyTransfer = (request: TransferRequest, onProgress: (progress: Progress) => void) =>
  callWithProgress<CopyView>('music_copy', { ...request }, onProgress);

export const tidyCard = () => call<{ removed: number; marker_written: boolean }>('music_tidy');
