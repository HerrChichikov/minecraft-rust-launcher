import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';

interface Channel<T> {
  event: string;
  snapshot: string;
  parse: (value: unknown) => T | null;
  connectionError: string;
}

export function useOperation<T extends { sequence: number }>(channel: Channel<T>) {
  const [status, setStatus] = useState<T | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [connectedAttempt, setConnectedAttempt] = useState(-1);
  const [failedAttempt, setFailedAttempt] = useState(-1);
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);
  const inFlight = useRef(false);
  const { parse, event, snapshot, connectionError } = channel;

  useEffect(() => {
    let active = true;
    let received = false;
    let cleanup: (() => void) | undefined;
    function accept(value: unknown) {
      const next = parse(value);
      if (next)
        setStatus((previous) => (previous && previous.sequence >= next.sequence ? previous : next));
    }
    void (async () => {
      try {
        cleanup = await listen<unknown>(event, ({ payload }) => {
          if (!active) return;
          try {
            accept(payload);
            if (!received) {
              received = true;
              setConnectedAttempt(attempt);
              setFailedAttempt(-1);
            }
          } catch {
            setError('Не удалось прочитать последнее обновление.');
          }
        });
        if (!active) {
          cleanup();
          return;
        }
        const initial = await invoke<unknown>(snapshot);
        if (active) {
          accept(initial);
          setConnectedAttempt(attempt);
        }
      } catch {
        if (active && !received) setFailedAttempt(attempt);
      }
    })();
    return () => {
      active = false;
      cleanup?.();
    };
  }, [attempt, event, parse, snapshot]);

  async function execute(
    action: () => Promise<unknown>,
    failure: (error: unknown) => string,
    returnsStatus = true,
  ) {
    if (inFlight.current) return;
    inFlight.current = true;
    setPending(true);
    setError(undefined);
    try {
      const value = await action();
      const next = returnsStatus ? parse(value) : null;
      if (next)
        setStatus((previous) => (previous && previous.sequence >= next.sequence ? previous : next));
    } catch (e: unknown) {
      setError(failure(e));
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }

  return {
    status,
    pending,
    connected: connectedAttempt === attempt,
    connectionError: failedAttempt === attempt ? connectionError : undefined,
    error,
    execute,
    reconnect: () => {
      setError(undefined);
      setAttempt((n) => n + 1);
    },
  };
}
