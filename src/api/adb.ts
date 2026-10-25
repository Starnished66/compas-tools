import { call, callWithProgress } from './invoke';
import type {
  BackupReport,
  DeviceView,
  InstallReport,
  LogBundle,
  Model,
  Progress,
  ReleaseKind,
  ToolsStatus,
} from './types';

type OnProgress = (progress: Progress) => void;

export const adbStatus = () => call<ToolsStatus>('adb_status');

export const downloadAdb = (onProgress: OnProgress) =>
  callWithProgress<ToolsStatus>('adb_download', {}, onProgress);

export const listDevices = () => call<DeviceView[]>('adb_devices');

export const backupDevice = (serial: string, onProgress: OnProgress) =>
  callWithProgress<BackupReport>('adb_backup', { serial }, onProgress);

export const installOnDevice = (
  serial: string,
  kind: ReleaseKind,
  model: Model,
  onProgress: OnProgress,
) => callWithProgress<InstallReport>('adb_install', { serial, kind, model }, onProgress);

export const collectLogs = (serial: string, onProgress: OnProgress) =>
  callWithProgress<LogBundle>('adb_collect_logs', { serial }, onProgress);

export const saveLogs = (path: string) => call<null>('adb_save_logs', { path });
