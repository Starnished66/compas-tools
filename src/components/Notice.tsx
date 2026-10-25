import type { ReactNode } from 'react';

import { Icon } from './Icon';
import type { IconName } from './icons';

const ICON: Record<Props['tone'], IconName> = {
  info: 'info',
  warning: 'warning',
  success: 'ok',
};

interface Props {
  tone: 'warning' | 'success' | 'info';
  /** Replaces the icon that goes with the tone. */
  icon?: IconName;
  children: ReactNode;
}

/** A non-blocking message. Errors use ErrorBanner so they are announced. */
export function Notice({ tone, icon, children }: Props) {
  return (
    <div className={`notice notice--${tone}`} role="status">
      <Icon name={icon ?? ICON[tone]} />
      <div className="notice__body stack">{children}</div>
    </div>
  );
}
