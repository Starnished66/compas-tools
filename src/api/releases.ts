import { call, callWithProgress } from './invoke';
import type { LatestRelease, Model, PlacedFile, Progress, ReleaseKind } from './types';

export const latestRelease = () => call<LatestRelease>('releases_latest');

export const placeRelease = (
  kind: ReleaseKind,
  model: Model,
  onProgress: (progress: Progress) => void,
) => callWithProgress<PlacedFile>('releases_place', { kind, model }, onProgress);

export const otherUpts = (kind: ReleaseKind, model: Model) =>
  call<string[]>('releases_other_upts', { kind, model });
