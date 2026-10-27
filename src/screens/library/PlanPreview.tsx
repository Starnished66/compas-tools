import type { IssueKind, PlanView } from '../../api/types';
import { Notice } from '../../components/Notice';
import { formatBytes } from '../../format';
import { t } from '../../i18n';

function issueText(kind: IssueKind): string {
  switch (kind) {
    case 'too_large_for_fat':
      return t('Too big for the player (limit 4 GiB per file)');
    case 'name_too_long':
      return t('A name is longer than 255 characters');
    case 'path_too_long':
      return t('The path is too long for the player (limit 600 bytes)');
    case 'name_conflict':
      return t('Another file ends up with the same name');
    case 'unreadable':
      return t('Cannot be read');
    case 'reserved_name':
      return t('A hidden name, a firmware image or something the player owns');
  }
}

const SAMPLE = 5;

export function PlanPreview({ plan }: { plan: PlanView }) {
  const missingSpace = plan.total_bytes > plan.free_bytes;
  return (
    <section className="panel stack" aria-label={t('What will be copied')}>
      <h2>{t('What will be copied')}</h2>
      <p>
        {t('{count} files, {size}; {free} free on the player.', {
          count: plan.files,
          size: formatBytes(plan.total_bytes),
          free: formatBytes(plan.free_bytes),
        })}
      </p>
      {plan.existing > 0 && (
        <p className="muted">
          {t(
            '{count} already on the player. Identical files are skipped; different ones are replaced.',
            {
              count: plan.existing,
            },
          )}
        </p>
      )}
      {plan.skipped_clutter > 0 && (
        <p className="muted">
          {t('{count} hidden files, macOS leftovers and firmware images are left out.', {
            count: plan.skipped_clutter,
          })}
        </p>
      )}
      {missingSpace && (
        <Notice tone="warning">
          <p>{t('There may not be enough free space for everything.')}</p>
        </Notice>
      )}
      {plan.renames.length > 0 && (
        <Notice tone="info">
          <p>
            {t('{count} names change because FAT does not allow them:', {
              count: plan.renames.length,
            })}
          </p>
          <ul>
            {plan.renames.slice(0, SAMPLE).map((r) => (
              <li key={r.from}>
                {r.from} → {r.to}
              </li>
            ))}
          </ul>
        </Notice>
      )}
      {plan.issues.length > 0 && (
        <Notice tone="warning">
          <p>{t('{count} files will not be copied:', { count: plan.issues.length })}</p>
          <ul>
            {plan.issues.slice(0, SAMPLE).map((issue) => (
              <li key={issue.source}>
                {issue.source}: {issueText(issue.kind)}
              </li>
            ))}
          </ul>
        </Notice>
      )}
      {plan.deletions_blocked && (
        <Notice tone="warning">
          <p>
            {t('Nothing will be removed from the player, because some files could not be read.')}
          </p>
        </Notice>
      )}
      {plan.deletions.length > 0 && (
        <Notice tone="warning">
          <p>
            {t('{count} files on the player are not in the folder and will be removed:', {
              count: plan.deletions.length,
            })}
          </p>
          <ul>
            {plan.deletions.slice(0, SAMPLE).map((path) => (
              <li key={path}>{path}</li>
            ))}
          </ul>
        </Notice>
      )}
    </section>
  );
}
