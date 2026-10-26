import { call, callWithProgress } from './invoke';
import type { LatestRelease, Model, PlacedFile, Progress, ReleaseKind } from './types';

/**
 * `refresh` skips the short-lived cache of the release. `installed` is a version read from
 * somewhere other than the selected player (over adb); without it the player's own is used.
 * `withoutDeviceInfo` says that player runs Compás but wrote no device.json (v1.2.0 and older).
 */
export const latestRelease = (refresh: boolean, installed?: string, withoutDeviceInfo?: boolean) =>
  call<LatestRelease>('releases_latest', { refresh, installed, withoutDeviceInfo });

export const placeRelease = (
  kind: ReleaseKind,
  model: Model,
  onProgress: (progress: Progress) => void,
) => callWithProgress<PlacedFile>('releases_place', { kind, model }, onProgress);

export const otherUpts = (kind: ReleaseKind, model: Model) =>
  call<string[]>('releases_other_upts', { kind, model });
