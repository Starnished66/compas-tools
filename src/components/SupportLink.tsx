import { openUrl } from '@tauri-apps/plugin-opener';

import { t } from '../i18n';

export const SUPPORT_URL = 'https://ko-fi.com/starnished';

/** A link that opens the project's Ko-fi page in the system browser. */
export function SupportLink({ className, label }: { className?: string; label?: string }) {
  return (
    <a
      className={className}
      href={SUPPORT_URL}
      onClick={(event) => {
        event.preventDefault();
        void openUrl(SUPPORT_URL);
      }}
    >
      {label ?? t('Support')}
    </a>
  );
}
