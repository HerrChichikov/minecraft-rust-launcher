import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type Event } from '@tauri-apps/api/event';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { GamePanel } from '../src/features/game/GamePanel';
import { workspaceFixture } from './fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

it('starts the selected profile and preserves the latest event across late replies', async () => {
  const profile = workspaceFixture.profiles[0];
  if (!profile) throw new Error('workspace fixture is missing the active profile');
  const base = {
    schemaVersion: 1,
    runId: '6b000000-0000-4000-8000-000000000000',
    profileName: profile.name,
    stage: 'preparing',
    sequence: 1,
    exit: null,
    errorCode: null,
    logSaved: null,
  };
  let event: ((event: Event<unknown>) => void) | undefined;
  let reply: ((value: unknown) => void) | undefined;
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(listen).mockImplementation((_name, handler) => {
    event = handler;
    return Promise.resolve(vi.fn());
  });
  vi.mocked(invoke).mockImplementation((command) => {
    if (command === 'get_workspace') return Promise.resolve(workspaceFixture);
    if (command === 'start_game')
      return new Promise((resolve) => {
        reply = resolve;
      });
    if (command === 'stop_game')
      return Promise.resolve({ ...base, sequence: 3, stage: 'stopping' });
    return Promise.resolve(null);
  });
  function emit(payload: unknown) {
    act(() => event?.({ event: 'launcher://game', id: 1, payload }));
  }

  render(<GamePanel available />);
  const play = screen.getByRole('button', { name: /Играть/ });
  await waitFor(() => expect(play).toBeEnabled());
  await userEvent.click(play);
  expect(invoke).toHaveBeenCalledWith('start_game', {
    request: {
      rootId: workspaceFixture.rootId,
      profileId: profile.id,
      profileRevision: profile.revision,
    },
  });
  emit({ ...base, stage: 'running', sequence: 2 });
  await act(() => {
    reply?.(base);
    return Promise.resolve();
  });
  expect(screen.getByText(/Minecraft запущен/)).toBeInTheDocument();
  expect(play).toBeDisabled();
  await userEvent.click(screen.getByRole('button', { name: 'Принудительно остановить' }));
  expect(invoke).toHaveBeenCalledWith('stop_game', { request: { runId: base.runId } });
  emit({
    ...base,
    sequence: 4,
    stage: 'finished',
    exit: { code: null, success: false, stopped: true },
    logSaved: true,
  });
  expect(screen.getByText(/Игра остановлена/)).toBeInTheDocument();
  expect(play).toBeEnabled();
});
