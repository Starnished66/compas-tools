import { useEffect, useRef } from 'react';

import { t } from '../i18n';
import { Button } from './Button';

interface Props {
  open: boolean;
  title: string;
  message: string;
  /** Lines listed under the message, such as the files a change would replace. */
  details?: string[];
  confirmLabel: string;
  cancelLabel?: string;
  destructive?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open,
  title,
  message,
  details = [],
  confirmLabel,
  cancelLabel,
  destructive = false,
  onConfirm,
  onCancel,
}: Props) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      className="dialog"
      aria-labelledby="confirm-title"
      onCancel={(event) => {
        event.preventDefault();
        onCancel();
      }}
    >
      <h2 id="confirm-title">{title}</h2>
      <p>{message}</p>
      {details.length > 0 && (
        <ul className="dialog__details">
          {details.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      )}
      <div className="dialog__actions">
        <Button icon="close" onClick={onCancel}>
          {cancelLabel ?? t('Cancel')}
        </Button>
        <Button
          variant={destructive ? 'danger' : 'primary'}
          icon={destructive ? 'remove' : 'ok'}
          onClick={onConfirm}
        >
          {confirmLabel}
        </Button>
      </div>
    </dialog>
  );
}
