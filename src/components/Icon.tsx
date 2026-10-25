import { ICONS, type IconName } from './icons';

const files = import.meta.glob<string>('../assets/icons/**/*.svg', {
  eager: true,
  query: '?url',
  import: 'default',
});

function url(theme: string, path: string): string {
  return files[`../assets/icons/${theme}/${path}.svg`] ?? '';
}

/** A Breeze icon, in its dark variant, or the light one when the system asks for light. */
export function Icon({ name }: { name: IconName }) {
  const path = ICONS[name];
  const size = Number(path.split('/')[1]);
  return (
    <picture className="icon">
      <source media="(prefers-color-scheme: light)" srcSet={url('breeze', path)} />
      <img src={url('breeze-dark', path)} width={size} height={size} alt="" />
    </picture>
  );
}
