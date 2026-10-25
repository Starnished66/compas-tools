import { t } from '../i18n';
import { Button } from './Button';

interface Props {
  message: string;
  onDismiss?: () => void;
}

export function ErrorBanner({ message, onDismiss }: Props) {
  return (
    <div className="notice notice--danger" role="alert">
      <span>{message}</span>
      {onDismiss && <Button onClick={onDismiss}>{t('Dismiss')}</Button>}
    </div>
  );
}
