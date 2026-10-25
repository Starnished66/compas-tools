import type { ReactNode } from 'react';

interface Props {
  title: string;
  children?: ReactNode;
}

export function EmptyState({ title, children }: Props) {
  return (
    <div className="empty-state">
      <h2>{title}</h2>
      {children}
    </div>
  );
}
