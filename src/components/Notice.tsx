import type { ReactNode } from 'react';

interface Props {
  tone: 'warning' | 'success' | 'info';
  children: ReactNode;
}

/** A non-blocking message. Errors use ErrorBanner so they are announced. */
export function Notice({ tone, children }: Props) {
  return (
    <div className={`notice notice--${tone}`} role="status">
      <div className="stack">{children}</div>
    </div>
  );
}
