import { useEffect, useState } from 'react';

import { errorMessage } from '../../api/invoke';
import { libraryStatus } from '../../api/library';
import type { LibraryStatus } from '../../api/types';
import { useCard } from '../../state/CardContext';

/** `compas-sdtool info` for the selected card; change `version` to read it again. */
export function useLibraryStatus(version = 0) {
  const { card } = useCard();
  const root = card?.root ?? null;
  const [status, setStatus] = useState<LibraryStatus | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    if (!root) return;
    let current = true;
    setStatus(null);
    setProblem(null);
    libraryStatus().then(
      (value) => {
        if (current) setStatus(value);
      },
      (failure: unknown) => {
        if (current) setProblem(errorMessage(failure));
      },
    );
    return () => {
      current = false;
    };
  }, [root, version]);

  return { status, problem };
}
