import { call } from './invoke';
import type { LatestRelease } from './types';

/**
 * `refresh` skips the short-lived cache of the release. `installed` is the version read from
 * the player over adb; without it the freshness is unknown.
 */
export const latestRelease = (refresh: boolean, installed?: string) =>
  call<LatestRelease>('releases_latest', { refresh, installed });
