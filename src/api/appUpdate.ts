import { call } from './invoke';

export interface AppUpdate {
  /** Whether this build can update itself at all. */
  enabled: boolean;
  version: string | null;
  notes: string | null;
}

export const checkAppUpdate = () => call<AppUpdate>('app_update_check');
export const installAppUpdate = () => call<AppUpdate>('app_update_install');
