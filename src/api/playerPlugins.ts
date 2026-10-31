import { call } from './invoke';
import type { Reply } from './settings';

export interface StorePlugin {
  id: string;
  name: string;
  version: string;
  state: string;
  incompatible: boolean;
  needs_confirmation: boolean;
  player_layout: boolean;
  theme_pack: boolean;
  description: string;
  category: string;
  author: string;
  size: number;
}
export interface StoreSnapshot {
  state: string;
  percent: number;
  current_plugin: string;
  error: string | null;
  changed: boolean;
  refresh_required: boolean;
  confirmation_required: boolean;
  offset: number;
  limit: number;
  total: number;
  plugins: StorePlugin[];
  accepted?: boolean;
}
export interface InstalledPlugin {
  id: string;
  name: string;
  enabled: boolean;
  loaded: boolean;
  error: string | null;
  player_layout: boolean;
  theme_package: boolean;
  can_open_settings?: boolean;
}
export interface InstalledSnapshot {
  offset: number;
  limit: number;
  total: number;
  truncated: boolean;
  refresh_pending: boolean;
  plugins: InstalledPlugin[];
}
export interface PlayerLayout {
  id: string;
  name: string;
  source: string;
  has_preview: boolean;
}
export interface LayoutSnapshot {
  current: string;
  offset: number;
  limit: number;
  total: number;
  layouts: PlayerLayout[];
}
export interface ActionResult {
  accepted: boolean;
  refresh_requested?: boolean;
  current?: string;
  state?: string;
  percent?: number;
  current_plugin?: string;
  error?: string | null;
  changed?: boolean;
  refresh_required?: boolean;
  confirmation_required?: boolean;
  offset?: number;
  limit?: number;
  total?: number;
  plugins?: StorePlugin[];
}
export interface PreviewResult {
  mime?: string;
  data?: string;
  pending?: boolean;
}

export interface PluginCapabilities {
  features: string[];
  settingsSchemaVersion?: number;
}
const page = (offset = 0, limit = 50) => ({ offset, limit });
export const pluginsCapabilities = (session: number) =>
  call<PluginCapabilities>('player_plugins_capabilities', { session });
export const pluginStore = (session: number, offset = 0, limit = 50) =>
  call<Reply<StoreSnapshot>>('player_plugin_store', { session, ...page(offset, limit) });
export const storePluginPreview = (session: number, id: string) =>
  call<Reply<PreviewResult>>('player_plugin_store_preview', { session, id });
export const refreshPluginStore = (session: number) =>
  call<Reply<ActionResult>>('player_plugin_store_refresh', { session });
export const installPlugin = (session: number, id: string, force = false) =>
  call<Reply<ActionResult>>('player_plugin_install', { session, id, force });
export const updatePlugin = (session: number, id: string, force = false) =>
  call<Reply<ActionResult>>('player_plugin_update', { session, id, force });
export const updateAllPlugins = (session: number) =>
  call<Reply<ActionResult>>('player_plugin_update_all', { session });
export const uninstallPlugin = (session: number, id: string) =>
  call<Reply<ActionResult>>('player_plugin_uninstall', { session, id });
export const installedPlugins = (session: number, offset = 0, limit = 50) =>
  call<Reply<InstalledSnapshot>>('player_plugins_installed', { session, ...page(offset, limit) });
export const setPluginEnabled = (session: number, id: string, enabled: boolean) =>
  call<Reply<ActionResult>>('player_plugin_set_enabled', { session, id, enabled });
export const refreshPlayerPlugins = (session: number) =>
  call<Reply<ActionResult>>('player_plugins_refresh', { session });
export const openPluginSettings = (session: number, id: string) =>
  call<Reply<ActionResult>>('player_plugin_open_settings', { session, id });
export const playerLayouts = (session: number, offset = 0, limit = 50) =>
  call<Reply<LayoutSnapshot>>('player_layouts', { session, ...page(offset, limit) });
export const playerLayoutPreview = (session: number, id: string) =>
  call<Reply<PreviewResult>>('player_layout_preview', { session, id });
export const selectPlayerLayout = (session: number, id: string) =>
  call<Reply<ActionResult>>('player_layout_select', { session, id });
