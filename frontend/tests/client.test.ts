import { invoke, isTauri } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getSnapshot, LauncherClientError, parseSnapshot } from '../src/shared/ipc/client';
import { foundationSnapshot } from './fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

describe('IPC trust boundary', () => {
  beforeEach(() => vi.mocked(isTauri).mockReturnValue(true));

  it('does not invent a backend response in a browser', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(getSnapshot()).rejects.toMatchObject({ code: 'desktop_required' });
    expect(invoke).not.toHaveBeenCalled();
  });

  it('reads the backend snapshot through the only registered command', async () => {
    vi.mocked(invoke).mockResolvedValue(foundationSnapshot);
    await expect(getSnapshot()).resolves.toEqual(foundationSnapshot);
    expect(invoke).toHaveBeenCalledWith('get_snapshot');
  });

  it.each([
    null,
    { ...foundationSnapshot, schemaVersion: 2 },
    { ...foundationSnapshot, features: { gameLaunch: 'yes' } },
    { ...foundationSnapshot, platform: { os: 'macos', architecture: 'unknown' } },
  ])('rejects an incompatible or malformed backend response', (value) => {
    expect(() => parseSnapshot(value)).toThrow(LauncherClientError);
  });

  it('replaces raw backend error text before it reaches UI state', async () => {
    vi.mocked(invoke).mockRejectedValue('secret-token /private/user/path');
    try {
      await getSnapshot();
      expect.unreachable();
    } catch (error) {
      expect(error).toMatchObject({ code: 'backend_unavailable' });
      expect(String(error)).not.toContain('secret-token');
      expect(String(error)).not.toContain('/private/user/path');
    }
  });
});
