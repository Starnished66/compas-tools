const ISSUES = 'https://github.com/Starnished66/compas-player/issues/new';

/** A pre-filled new-issue page. Nothing is uploaded: the zip is attached by hand. */
export function issueUrl(title: string, summary: string): string {
  const body = [
    '**What happened**',
    '',
    '(describe the problem here)',
    '',
    '**Device**',
    '',
    summary,
    '',
    '_The log report saved by Compás Tools is attached to this issue._',
  ].join('\n');
  const params = new URLSearchParams({ title, body });
  return `${ISSUES}?${params.toString()}`;
}
