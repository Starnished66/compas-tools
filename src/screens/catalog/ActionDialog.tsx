import { ConfirmDialog } from '../../components/ConfirmDialog';
import { t } from '../../i18n';
import { useCatalog } from '../../state/CatalogContext';

/** The confirmation for plugin changes; mounted once so it outlives view switches. */
export function ActionDialog() {
  const {
    entries,
    actions: { dialog, confirm, cancelDialog },
  } = useCatalog();
  const name = (id: string) => entries.find((e) => e.id === id)?.name ?? id;

  let title = t('Replace local files?');
  let message = '';
  let details: string[] = [];
  let confirmLabel = t('Yes');
  let cancelLabel: string | undefined = t('No');
  if (dialog?.type === 'replace') {
    message = t('{name} would replace these files on the player:', { name: name(dialog.id) });
    details = dialog.files;
  } else if (dialog?.type === 'replace_all') {
    message = t('These plugins would replace local files on the player:');
    details = dialog.items.map((item) => `${name(item.id)}: ${item.files.join(', ')}`);
  } else if (dialog?.type === 'remove') {
    title = t('Remove this plugin?');
    message = t('{name} and its files will be removed from the player.', { name: name(dialog.id) });
    confirmLabel = t('Remove');
    cancelLabel = undefined;
  }

  return (
    <ConfirmDialog
      open={dialog !== null}
      title={title}
      message={message}
      details={details}
      confirmLabel={confirmLabel}
      cancelLabel={cancelLabel}
      destructive
      onConfirm={() => void confirm()}
      onCancel={cancelDialog}
    />
  );
}
