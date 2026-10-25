import { call, callWithProgress } from './invoke';
import type { ArtDone, LibraryStatus, Progress, ScanDone } from './types';

export const libraryStatus = () => call<LibraryStatus>('library_info');

export const scanLibrary = (full: boolean, onProgress: (progress: Progress) => void) =>
  callWithProgress<ScanDone>('library_scan', { full }, onProgress);

export const buildArt = (onProgress: (progress: Progress) => void) =>
  callWithProgress<ArtDone>('library_art', {}, onProgress);
