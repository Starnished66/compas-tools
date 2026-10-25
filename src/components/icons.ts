/**
 * The Breeze icons the app uses: a name for the code, and the icon's path under
 * `assets/icons/<theme>/`. The folder name is the pixel size (sidebar 22, buttons 16).
 */
export const ICONS = {
  card: 'devices/22/media-flash-sd-mmc',
  music: 'places/22/folder-music',
  library: 'places/22/server-database',
  plugins: 'actions/22/plugins',
  layouts: 'actions/22/view-grid',
  themes: 'preferences/22/preferences-desktop-theme-global-symbolic',
  updates: 'status/22/system-software-update',
  install: 'actions/16/download',
  update: 'actions/16/go-up',
  refresh: 'actions/16/view-refresh',
  uninstall: 'actions/16/edit-delete',
  remove: 'actions/16/edit-delete-remove',
  close: 'actions/16/window-close',
  search: 'actions/16/edit-find',
  addFiles: 'actions/16/list-add',
  addFolder: 'actions/16/document-open-folder',
  chooseFolder: 'places/16/folder-open',
  copy: 'actions/16/edit-copy',
  eject: 'actions/16/media-eject',
  tags: 'actions/16/tag',
  albumArt: 'actions/16/viewimage',
  ok: 'actions/16/dialog-ok-apply',
  info: 'status/16/dialog-information',
  warning: 'status/16/dialog-warning',
  error: 'status/16/dialog-error',
} as const;

export type IconName = keyof typeof ICONS;
