/** Long text that keeps its beginning and end visible, with the full text as a tooltip. */
export function MiddleEllipsis({ text, tail = 24 }: { text: string; tail?: number }) {
  const split = Math.max(text.length - tail, 0);
  return (
    <span className="middle-ellipsis" title={text}>
      <span className="middle-ellipsis__start">{text.slice(0, split)}</span>
      <span className="middle-ellipsis__end">{text.slice(split)}</span>
    </span>
  );
}
