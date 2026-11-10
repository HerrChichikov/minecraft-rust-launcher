import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type Event } from '@tauri-apps/api/event';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { App } from '../src/app/App';
import authContract from '../../tests/contracts/auth.v1.json';
import { foundationSnapshot, workspaceFixture } from './fixtures';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

const channels = new Map<string, (event: Event<unknown>) => void>();
const response = (command: string) => {
  if (command === 'get_snapshot') return foundationSnapshot;
  if (command === 'get_workspace') return workspaceFixture;
  if (command === 'get_auth') return authContract;
  return null;
};

describe('launcher screens', () => {
  beforeEach(() => {
    channels.clear();
    vi.mocked(listen).mockClear();
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockImplementation((command) => Promise.resolve(response(command)));
    vi.mocked(listen).mockImplementation((name, handler) => {
      channels.set(name, handler);
      return Promise.resolve(() => channels.delete(name));
    });
  });

  it('shows real loading and uses the sign-in screen when the guest wants to play', async () => {
    let resolve: ((value: unknown) => void) | undefined;
    vi.mocked(invoke).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done;
      }),
    );
    render(<App />);
    expect(screen.getByRole('status')).toHaveTextContent('Подключение к ядру');
    expect(screen.queryByRole('button', { name: 'Войти для игры' })).not.toBeInTheDocument();

    resolve?.(foundationSnapshot);
    const play = await screen.findByRole('button', { name: 'Войти для игры' });
    await waitFor(() => expect(play).toBeEnabled());
    await userEvent.click(play);
    expect(screen.getByRole('heading', { level: 1, name: 'Аккаунт' })).toHaveFocus();
    expect(screen.getByRole('button', { name: 'Аккаунт' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByLabelText('Пароль Ely.by')).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith('start_game', expect.anything());
    await userEvent.type(screen.getByLabelText('Пароль Ely.by'), 'draft-fixture-password');
    await userEvent.click(screen.getByRole('button', { name: 'Настройки' }));
    await userEvent.click(screen.getByRole('button', { name: 'Аккаунт' }));
    expect(screen.getByLabelText('Пароль Ely.by')).toHaveValue('');
  });

  it('retains installer progress and blocks settings edits across screen changes', async () => {
    render(<App />);
    await waitFor(() => expect(channels.size).toBe(2));
    await userEvent.click(screen.getByRole('button', { name: 'Настройки' }));
    expect(screen.getByRole('heading', { level: 1, name: 'Настройки' })).toHaveFocus();
    const ram = await screen.findByLabelText('Максимум RAM, МиБ');
    const event = 'launcher://installation';
    act(() =>
      channels.get(event)?.({
        event,
        id: 1,
        payload: {
          schemaVersion: 1,
          operationId: 'operation-1',
          sequence: 7,
          rootId: workspaceFixture.rootId,
          profileId: workspaceFixture.activeProfileId,
          profileName: 'Основная конфигурация',
          stage: 'downloading',
          completedBytes: 40,
          totalBytes: 100,
          elapsedMs: 500,
          downloadedBytes: 40,
          terminal: false,
          errorCode: null,
        },
      }),
    );
    expect(ram).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: /^Игра/ }));
    expect(screen.getByRole('progressbar')).toHaveValue(40);
    expect(screen.getByRole('button', { name: 'Отменить' })).toBeEnabled();
    expect(vi.mocked(listen)).toHaveBeenCalledTimes(2);
  });

  it('recovers from a backend failure on explicit retry', async () => {
    vi.mocked(invoke).mockRejectedValueOnce('private backend details');
    render(<App />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось подключиться');
    expect(screen.queryByText('private backend details')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Повторить подключение' }));
    expect(await screen.findByRole('heading', { name: 'Minecraft' })).toBeInTheDocument();
  });

  it('shows a clear browser-only state', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    vi.mocked(invoke).mockClear();
    render(<App />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Откройте desktop-приложение');
    expect(invoke).not.toHaveBeenCalled();
  });
});
