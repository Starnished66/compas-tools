import { call, callWithProgress } from './invoke';
import type {
  BackupReport,
  DeviceInfo,
  DeviceView,
  InstallReport,
  LogBundle,
  Progress,
  ReleaseKind,
  ToolsStatus,
} from './types';

type OnProgress = (progress: Progress) => void;

export const adbStatus = () => call<ToolsStatus>('adb_status');

export const downloadAdb = (onProgress: OnProgress) =>
  callWithProgress<ToolsStatus>('adb_download', {}, onProgress);

export const listDevices = () => call<DeviceView[]>('adb_devices');

/**
 * The device list alone, without asking any device anything (`info` is there only for players
 * identified before). Cheap enough to poll; `identifyDevice` fills the rest in.
 */
export const listDevicesQuick = () => call<DeviceView[]>('adb_devices_quick');

/** Identifies one player; the backend keeps the answer until the player disconnects. */
export const identifyDevice = (serial: string) => call<DeviceInfo>('adb_identify', { serial });

/** Stops and starts the adb server again. */
export const restartAdbServer = () => call<null>('adb_restart_server');

export const backupDevice = (serial: string, onProgress: OnProgress) =>
  callWithProgress<BackupReport>('adb_backup', { serial }, onProgress);

export const installOnDevice = (serial: string, kind: ReleaseKind, onProgress: OnProgress) =>
  callWithProgress<InstallReport>('adb_install', { serial, kind }, onProgress);

export const collectLogs = (serial: string, onProgress: OnProgress) =>
  callWithProgress<LogBundle>('adb_collect_logs', { serial }, onProgress);

export const saveLogs = (path: string) => call<null>('adb_save_logs', { path });

/** Looks at a device that did not look like the player, because the user asked. */
export const probeDevice = (serial: string) => call<DeviceInfo>('adb_probe', { serial });

export const removeUpdateFile = (serial: string, remotePath: string) =>
  call<null>('adb_remove_update_file', { serial, remotePath });
