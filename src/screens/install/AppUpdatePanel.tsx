import { getVersion } from '@tauri-apps/api/app';
import { useCallback, useEffect, useRef, useState } from 'react';

import {
  checkAppUpdate,
  installAppUpdate,
  type AppUpdate,
  type UpdateChannel,
} from '../../api/appUpdate';
import { errorMessage } from '../../api/invoke';
import type { Progress } from '../../api/types';
import { Button } from '../../components/Button';
import { ErrorBanner } from '../../components/ErrorBanner';
import { Notice } from '../../components/Notice';
import { ProgressBar } from '../../components/ProgressBar';
import { t } from '../../i18n';

const CHECK_COOLDOWN_MS = 6 * 60 * 60 * 1000;
const RETRY_BACKOFF_MS = 15 * 60 * 1000;
const CHANNEL_KEY = 'compas-tools-app-update-channel';
const RESULT_KEY = 'compas-tools-app-update-result';

function channelStorageKey(key: string, channel: UpdateChannel): string {
  return `${key}-${channel}`;
}

function savedChannel(): UpdateChannel {
  try {
    return localStorage.getItem(CHANNEL_KEY) === 'beta' ? 'beta' : 'stable';
  } catch {
    return 'stable';
  }
}

function storedTime(key: string): number {
  try {
    const value = Number(localStorage.getItem(key));
    return Number.isFinite(value) && value >= 0 && value <= Date.now() ? value : 0;
  } catch {
    return 0;
  }
}

function saveTime(key: string, time: number) {
  try {
    localStorage.setItem(key, String(time));
  } catch {
    // Keep update checks available in restricted webviews even when storage is unavailable.
  }
}

function cachedUpdate(currentVersion: string, channel: UpdateChannel): AppUpdate | null {
  const resultKey = channelStorageKey(RESULT_KEY, channel);
  try {
    const value: unknown = JSON.parse(localStorage.getItem(resultKey) ?? 'null');
    if (
      typeof value === 'object' &&
      value !== null &&
      'enabled' in value &&
      typeof value.enabled === 'boolean' &&
      'current_version' in value &&
      value.current_version === currentVersion &&
      'channel' in value &&
      value.channel === channel &&
      'version' in value &&
      (typeof value.version === 'string' || value.version === null) &&
      'notes' in value &&
      (typeof value.notes === 'string' || value.notes === null)
    ) {
      return value as AppUpdate;
    }
    if (value !== null) {
      localStorage.removeItem(resultKey);
      localStorage.removeItem(channelStorageKey('compas-tools-app-update-last-success', channel));
      localStorage.removeItem(channelStorageKey('compas-tools-app-update-last-attempt', channel));
    }
  } catch {
    // Ignore malformed or unavailable local storage and let the check run.
  }
  return null;
}

/** Checks on startup and resume with a six hour cooldown; users can also check manually. */
export function AppUpdatePanel({ visible = true }: { visible?: boolean }) {
  const [channel, setChannel] = useState<UpdateChannel>(savedChannel);
  const [update, setUpdate] = useState<AppUpdate | null>(null);
  const [checkFinished, setCheckFinished] = useState(false);
  const [checkError, setCheckError] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);
  const [installed, setInstalled] = useState(false);
  const checkingRef = useRef(false);
  const installingRef = useRef(false);
  const currentVersionRef = useRef<string | null>(null);
  const channelRef = useRef(channel);

  const check = useCallback(async (manual: boolean, requestedChannel = channelRef.current) => {
    if (checkingRef.current || installingRef.current) return;
    checkingRef.current = true;
    setChecking(true);
    setCheckError(null);
    try {
      const currentVersion = (currentVersionRef.current ??= await getVersion());
      const cached = cachedUpdate(currentVersion, requestedChannel);
      const now = Date.now();
      const lastSuccessKey = channelStorageKey(
        'compas-tools-app-update-last-success',
        requestedChannel,
      );
      const lastAttemptKey = channelStorageKey(
        'compas-tools-app-update-last-attempt',
        requestedChannel,
      );
      if (!manual && cached && now - storedTime(lastSuccessKey) < CHECK_COOLDOWN_MS) {
        setUpdate(cached);
        return;
      }
      if (!manual && now - storedTime(lastAttemptKey) < RETRY_BACKOFF_MS) {
        setUpdate(cached);
        return;
      }

      setUpdate(cached);
      saveTime(lastAttemptKey, now);
      const result = await checkAppUpdate(requestedChannel);
      if (result.current_version !== currentVersion || result.channel !== requestedChannel) {
        currentVersionRef.current = result.current_version;
        throw new Error(t('The update response did not match the selected channel.'));
      }
      setUpdate(result);
      saveTime(lastSuccessKey, Date.now());
      try {
        localStorage.setItem(
          channelStorageKey(RESULT_KEY, requestedChannel),
          JSON.stringify(result),
        );
      } catch {
        // Keep the live check result even when webview storage is unavailable.
      }
    } catch (error) {
      setCheckError(errorMessage(error));
    } finally {
      checkingRef.current = false;
      setChecking(false);
      setCheckFinished(true);
    }
  }, []);

  useEffect(() => {
    void check(false);
    const onResume = () => {
      if (document.visibilityState === 'visible' && !installingRef.current) void check(false);
    };
    document.addEventListener('visibilitychange', onResume);
    window.addEventListener('focus', onResume);
    return () => {
      document.removeEventListener('visibilitychange', onResume);
      window.removeEventListener('focus', onResume);
    };
  }, [check]);

  const install = async () => {
    const version = update?.version;
    if (!version || installingRef.current) return;
    installingRef.current = true;
    setInstalling(true);
    setProgress(null);
    setInstallError(null);
    try {
      const result = await installAppUpdate(version, channel, setProgress);
      if (result.version === version && result.channel === channel) {
        setInstalled(true);
      } else {
        setUpdate(result);
      }
    } catch (error) {
      setInstallError(errorMessage(error));
    } finally {
      installingRef.current = false;
      setInstalling(false);
    }
  };

  if ((!update?.enabled && !checkFinished) || update?.enabled === false) return null;

  return (
    <section
      className="panel stack"
      aria-label={t('Compás Tools updates')}
      hidden={!visible && !update?.version && !installed}
    >
      <h2>{t('Compás Tools updates')}</h2>
      {checkError && (
        <ErrorBanner
          message={checkError}
          onDismiss={() => {
            setCheckError(null);
          }}
        />
      )}
      {installError && (
        <ErrorBanner
          message={installError}
          onDismiss={() => {
            setInstallError(null);
          }}
        />
      )}
      <label className="row">
        <span>{t('Update channel')}</span>
        <select
          aria-label={t('Update channel')}
          value={channel}
          disabled={checking || installing}
          onChange={(event) => {
            const selected = event.currentTarget.value as UpdateChannel;
            setChannel(selected);
            channelRef.current = selected;
            try {
              localStorage.setItem(CHANNEL_KEY, selected);
            } catch {
              // The channel selection still applies for this session.
            }
            void check(true, selected);
          }}
        >
          <option value="stable">{t('Stable releases')}</option>
          <option value="beta">{t('Beta releases')}</option>
        </select>
      </label>
      {installed ? (
        <Notice tone="success">
          <p>{t('Update installed. Compás Tools is restarting.')}</p>
        </Notice>
      ) : update?.version ? (
        <>
          <Notice tone="info">
            <p>{t('Compás Tools {version} is available.', { version: update.version })}</p>
            {update.notes && <p className="release-notes">{update.notes}</p>}
          </Notice>
          {installing && (
            <ProgressBar progress={progress} label={t('Downloading Compás Tools update')} />
          )}
          <div className="row">
            <Button
              variant="primary"
              disabled={checking || installing}
              onClick={() => void install()}
            >
              {installing ? t('Installing update…') : t('Download and install')}
            </Button>
            <Button disabled={checking || installing} onClick={() => void check(true)}>
              {checking ? t('Checking…') : t('Check for updates')}
            </Button>
          </div>
        </>
      ) : (
        <div className="row">
          <p className="muted">
            {checking
              ? t('Checking for updates…')
              : checkError
                ? t('Could not check for updates.')
                : update
                  ? t('Compás Tools is up to date.')
                  : t('Update status is unavailable.')}
          </p>
          <Button disabled={checking || installing} onClick={() => void check(true)}>
            {checking ? t('Checking…') : t('Check for updates')}
          </Button>
        </div>
      )}
    </section>
  );
}
