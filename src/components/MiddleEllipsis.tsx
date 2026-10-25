/** Longest ending kept whole when cut at a folder boundary. */
const MAX_END = 48;

function cutPoint(text: string): number {
  const last = text.lastIndexOf('/');
  const previous = last > 0 ? text.lastIndexOf('/', last - 1) : -1;
  for (const slash of [previous, last]) {
    if (slash > 0 && text.length - slash <= MAX_END) return slash;
  }
  return Math.max(text.length - 24, 0);
}

/**
 * Long path that keeps its beginning and its last folder and file name visible,
 * cutting at a `/` rather than in the middle of a name. The full text is the tooltip.
 */
export function MiddleEllipsis({ text }: { text: string }) {
  const split = cutPoint(text);
  return (
    <span className="middle-ellipsis" title={text}>
      <span className="middle-ellipsis__start">{text.slice(0, split)}</span>
      <span className="middle-ellipsis__end">{text.slice(split)}</span>
    </span>
  );
}
