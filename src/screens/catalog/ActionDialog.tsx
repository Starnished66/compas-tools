import { ConfirmDialog } from '../../components/ConfirmDialog';
import { t } from '../../i18n';
import { useCatalog } from '../../state/CatalogContext';

/** The confirmation for plugin changes; mounted once so it outlives view switches. */
export function ActionDialog() {
  const {
    entries,
    actions: { dialog, confirm, cancelDialog },
  } = useCatalog();
  const name = entries.find((e) => e.id === dialog?.id)?.name ?? '';

  let title = t('Replace a local file?');
  let message = t('{name} would replace a file on the player that the store did not install.', {
    name,
  });
  let confirmLabel = t('Replace');
  if (dialog?.type === 'remove') {
    title = t('Remove this plugin?');
    message = t('{name} and its files will be removed from the player.', { name });
    confirmLabel = t('Remove');
  } else if (dialog?.kind === 'update') {
    message = t('Updating {name} would replace a file you changed.', { name });
  }

  return (
    <ConfirmDialog
      open={dialog !== null}
      title={title}
      message={message}
      confirmLabel={confirmLabel}
      destructive
      onConfirm={() => void confirm()}
      onCancel={cancelDialog}
    />
  );
}
