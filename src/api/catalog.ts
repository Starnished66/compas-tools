import { call, callWithProgress } from './invoke';
import type { Outcome, PluginEntry, Progress, UpdateAllReport } from './types';

type OnProgress = (progress: Progress) => void;

export const loadEntries = (refresh: boolean) =>
  call<PluginEntry[]>('catalog_entries', { refresh });

export const installPlugin = (pluginId: string, force: boolean, onProgress: OnProgress) =>
  callWithProgress<Outcome>('catalog_install', { pluginId, force }, onProgress);

export const updatePlugin = (pluginId: string, force: boolean, onProgress: OnProgress) =>
  callWithProgress<Outcome>('catalog_update', { pluginId, force }, onProgress);

export const updateAllPlugins = (onProgress: OnProgress) =>
  callWithProgress<UpdateAllReport>('catalog_update_all', {}, onProgress);

export const uninstallPlugin = (pluginId: string, onProgress: OnProgress) =>
  callWithProgress<null>('catalog_uninstall', { pluginId }, onProgress);

export const setPluginEnabled = (script: string, enabled: boolean) =>
  call<null>('catalog_set_enabled', { script, enabled });

/** Fetches the verified preview image and returns a blob URL for it. */
export async function loadPreviewUrl(pluginId: string): Promise<string> {
  const bytes = await call<ArrayBuffer>('catalog_preview', { pluginId });
  return URL.createObjectURL(new Blob([bytes], { type: 'image/jpeg' }));
}
