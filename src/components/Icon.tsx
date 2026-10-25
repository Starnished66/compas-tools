import { ICONS, type IconName } from './icons';

const files = import.meta.glob<string>('../assets/icons/**/*.svg', {
  eager: true,
  query: '?url',
  import: 'default',
});

function url(theme: string, path: string): string {
  return files[`../assets/icons/${theme}/${path}.svg`] ?? '';
}

/** A Breeze icon, in its light or dark variant to match the colour scheme. */
export function Icon({ name }: { name: IconName }) {
  const path = ICONS[name];
  const size = Number(path.split('/')[1]);
  return (
    <picture className="icon">
      <source media="(prefers-color-scheme: dark)" srcSet={url('breeze-dark', path)} />
      <img src={url('breeze', path)} width={size} height={size} alt="" />
    </picture>
  );
}
