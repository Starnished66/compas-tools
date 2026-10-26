import type { Model } from '../api/types';
import { t } from '../i18n';

const MODELS: { value: Model; label: string }[] = [
  { value: 'r1', label: 'Hiby R1' },
  { value: 'r3proii', label: 'Hiby R3 Pro II' },
  { value: 'r3ii_2025', label: 'Hiby R3 II (2025)' },
];

interface Props {
  value: Model | null;
  onChange: (model: Model) => void;
}

export function ModelPicker({ value, onChange }: Props) {
  return (
    <label className="stack">
      <span>{t('Which player is this?')}</span>
      <select
        className="field"
        value={value ?? ''}
        onChange={(event) => {
          onChange(event.target.value as Model);
        }}
      >
        <option value="" disabled>
          {t('Choose a model…')}
        </option>
        {MODELS.map((m) => (
          <option key={m.value} value={m.value}>
            {m.label}
          </option>
        ))}
      </select>
    </label>
  );
}
