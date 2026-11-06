import { useEffect, useState } from 'react';

import { getSnapshot, LauncherClientError } from '../../shared/ipc/client';
import type { ClientErrorCode, FoundationSnapshot } from '../../shared/ipc/types';

export type FoundationState =
  | { status: 'loading' }
  | { status: 'ready'; snapshot: FoundationSnapshot }
  | { status: 'error'; code: ClientErrorCode };

export function useFoundation(): { state: FoundationState; retry: () => void } {
  const [state, setState] = useState<FoundationState>({ status: 'loading' });
  const [request, setRequest] = useState(0);

  useEffect(() => {
    let active = true;
    void getSnapshot().then(
      (snapshot) => {
        if (active) setState({ status: 'ready', snapshot });
      },
      (error: unknown) => {
        if (active) {
          setState({
            status: 'error',
            code: error instanceof LauncherClientError ? error.code : 'backend_unavailable',
          });
        }
      },
    );
    return () => {
      active = false;
    };
  }, [request]);

  function retry() {
    setState({ status: 'loading' });
    setRequest((previous) => previous + 1);
  }

  return { state, retry };
}
