import { Button } from '../../components/Button';
import { Icon } from '../../components/Icon';
import { t } from '../../i18n';

interface Props {
  query: string;
  category: string;
  categories: string[];
  updateCount: number;
  busy: boolean;
  onQuery: (query: string) => void;
  onCategory: (category: string) => void;
  onUpdateAll: () => void;
  onRefresh: () => void;
}

export function Toolbar({
  query,
  category,
  categories,
  updateCount,
  busy,
  onQuery,
  onCategory,
  onUpdateAll,
  onRefresh,
}: Props) {
  return (
    <div className="toolbar" role="search">
      <div className="search toolbar__search">
        <Icon name="search" />
        <input
          className="field"
          type="search"
          value={query}
          placeholder={t('Search')}
          aria-label={t('Search')}
          onChange={(event) => {
            onQuery(event.target.value);
          }}
        />
      </div>
      <select
        className="field"
        value={category}
        aria-label={t('Category')}
        onChange={(event) => {
          onCategory(event.target.value);
        }}
      >
        <option value="">{t('All categories')}</option>
        {categories.map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </select>
      <Button icon="refresh" onClick={onRefresh} disabled={busy}>
        {t('Refresh')}
      </Button>
      <Button
        variant="primary"
        icon="update"
        onClick={onUpdateAll}
        disabled={busy || updateCount === 0}
      >
        {t('Update all ({count})', { count: updateCount })}
      </Button>
    </div>
  );
}
