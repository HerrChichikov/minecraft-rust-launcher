import { useEffect, useRef, useState } from 'react';
import { getWorkspace, StorageError, type Workspace } from './client';

function errorMessage(error: unknown) {
  return error instanceof StorageError ? error.message : new StorageError().message;
}

export function useWorkspace() {
  const [workspace, setWorkspace] = useState<Workspace>();
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [busy, setBusy] = useState(true);
  const [rootPath, setRootPath] = useState('');
  const mounted = useRef(false);
  const pending = useRef(false);
  useEffect(() => {
    let active = true;
    mounted.current = true;
    void getWorkspace().then(
      (w) => {
        if (active) {
          setWorkspace(w);
          setRootPath(w.dataRoot);
          setBusy(false);
        }
      },
      (e: unknown) => {
        if (active) {
          setError(errorMessage(e));
          setBusy(false);
        }
      },
    );
    return () => {
      active = false;
      mounted.current = false;
    };
  }, []);
  async function run(action: () => Promise<Workspace>, success?: string) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const next = await action();
      if (mounted.current) {
        setNotice(success);
        setWorkspace(next);
        setRootPath(next.dataRoot);
      }
    } catch (e: unknown) {
      if (mounted.current) setError(errorMessage(e));
    } finally {
      pending.current = false;
      if (mounted.current) setBusy(false);
    }
  }
  return { workspace, busy, error, notice, rootPath, setRootPath, run };
}
