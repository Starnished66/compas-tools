import { call, callWithProgress } from './invoke';
import type { Progress } from './types';

export type UpdateChannel = 'stable' | 'beta';

export interface AppUpdate {
  /** Whether this build can update itself at all. */
  enabled: boolean;
  version: string | null;
  notes: string | null;
  /** The installed app version, used to invalidate cached update results after restart. */
  current_version: string;
  channel: UpdateChannel;
}

export const checkAppUpdate = (channel: UpdateChannel) =>
  call<AppUpdate>('app_update_check', { channel });
export const installAppUpdate = (
  version: string,
  channel: UpdateChannel,
  onProgress: (progress: Progress) => void,
) =>
  callWithProgress<AppUpdate>(
    'app_update_install',
    { expectedVersion: version, channel },
    onProgress,
  );
