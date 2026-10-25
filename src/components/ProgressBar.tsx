import { progressLabel } from '../format';
import type { Progress } from '../api/types';

interface Props {
  progress: Progress | null;
  /** Accessible name of the operation, e.g. the plugin name. */
  label: string;
}

export function ProgressBar({ progress, label }: Props) {
  const determinate = progress !== null && progress.total > 0;
  const percent = determinate ? Math.round((progress.done / progress.total) * 100) : undefined;
  return (
    <div>
      <div
        className={`progress ${determinate ? '' : 'progress--indeterminate'}`}
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
      >
        <div className="progress__bar" style={determinate ? { width: `${percent}%` } : undefined} />
      </div>
      {progress && <p className="progress-label">{progressLabel(progress)}</p>}
    </div>
  );
}
