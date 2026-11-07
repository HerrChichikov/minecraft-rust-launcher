import { invoke } from '@tauri-apps/api/core';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthPanel } from '../src/features/auth/AuthPanel';
import { getAuth, parseAuth } from '../src/features/auth/client';
import contract from '../../tests/contracts/auth.v1.json';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: () => true }));
const production = parseAuth(contract);
const development = parseAuth({
  ...contract,
  providers: [
    ...contract.providers,
    {
      id: 'development-mock',
      available: true,
      development: true,
      flows: ['development_nickname'],
      unavailableReason: null,
    },
  ],
});
const authenticated = parseAuth({
  ...development,
  revision: 1,
  account: {
    providerId: 'development-mock',
    playerId: 'b50ad385-829d-3141-a216-7e7d7539ba7f',
    playerName: 'Notch',
    development: true,
  },
});
describe('authentication UI and protocol', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });
  it('does not manufacture production login or offer mock without backend capability', async () => {
    vi.mocked(invoke).mockResolvedValue(production);
    render(<AuthPanel />);
    expect(await screen.findByText('Ely.by: реальный вход пока недоступен.')).toBeInTheDocument();
    expect(screen.queryByLabelText('Тестовый ник')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Выйти' })).not.toBeInTheDocument();
  });
  it('uses backend-owned identity, reconnects and logs out by revision', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(development)
      .mockResolvedValueOnce(authenticated)
      .mockResolvedValueOnce(authenticated)
      .mockResolvedValueOnce({ ...production, providers: development.providers, revision: 2 });
    const view = render(<AuthPanel />);
    const input = await screen.findByLabelText('Тестовый ник');
    await userEvent.type(input, 'Notch');
    await userEvent.click(screen.getByRole('button', { name: 'Войти в Development / Mock' }));
    expect(await screen.findByText('Notch')).toBeInTheDocument();
    expect(screen.getByText(/Development \/ Mock —/)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('login', {
      request: { expectedRevision: 0, providerId: 'development-mock', nickname: 'Notch' },
    });
    view.unmount();
    render(<AuthPanel />);
    expect(await screen.findByText('Notch')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Выйти' }));
    expect(await screen.findByLabelText('Тестовый ник')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('logout', { request: { expectedRevision: 1 } });
  });
  it('shows pending without false success and sanitizes rejection', async () => {
    let reject: ((reason: unknown) => void) | undefined;
    vi.mocked(invoke)
      .mockResolvedValueOnce(development)
      .mockReturnValueOnce(
        new Promise((_resolve, fail) => {
          reject = fail;
        }),
      );
    render(<AuthPanel />);
    await userEvent.type(await screen.findByLabelText('Тестовый ник'), 'Notch');
    await userEvent.click(screen.getByRole('button', { name: 'Войти в Development / Mock' }));
    expect(screen.getByRole('button', { name: 'Выполняется вход…' })).toBeDisabled();
    expect(screen.queryByText(/Выполнен тестовый вход/)).not.toBeInTheDocument();
    reject?.({ code: 'revision_conflict', message: 'private-token-secret' });
    expect(await screen.findByRole('alert')).toHaveTextContent('Состояние сессии изменилось');
    expect(screen.queryByText('private-token-secret')).not.toBeInTheDocument();
  });
  it('rejects malformed or spoofed provider capabilities and removes extra secrets', () => {
    expect(() => parseAuth({ ...production, account: authenticated.account })).toThrow();
    expect(() =>
      parseAuth({
        ...development,
        providers: [...development.providers, development.providers[1]],
      }),
    ).toThrow();
    expect(() => parseAuth({ ...production, revision: -1 })).toThrow();
    expect(() =>
      parseAuth({ ...production, providers: [{ ...production.providers[0], available: true }] }),
    ).toThrow();
    expect(
      JSON.stringify(
        parseAuth({
          ...authenticated,
          accessToken: 'must-discard',
          account: { ...authenticated.account, accessToken: 'also-discard' },
        }),
      ),
    ).not.toContain('discard');
  });
  it('does not roll a refreshed session back when initial snapshot arrives late', async () => {
    let resolve: ((value: unknown) => void) | undefined;
    vi.mocked(invoke)
      .mockReturnValueOnce(
        new Promise((done) => {
          resolve = done;
        }),
      )
      .mockResolvedValueOnce(authenticated);
    render(<AuthPanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Обновить состояние входа' }));
    expect(await screen.findByText('Notch')).toBeInTheDocument();
    await act(async () => {
      resolve?.(development);
      await Promise.resolve();
    });
    expect(screen.getByText('Notch')).toBeInTheDocument();
    expect(screen.queryByLabelText('Тестовый ник')).not.toBeInTheDocument();
  });
  it('maps unknown backend errors without exposing their text', async () => {
    vi.mocked(invoke).mockRejectedValue({ code: 'unexpected', message: 'private-token' });
    await expect(getAuth()).rejects.toMatchObject({ code: 'auth_failed' });
  });
  it('lets the user recover a failed initial snapshot', async () => {
    vi.mocked(invoke).mockRejectedValueOnce('private').mockResolvedValueOnce(production);
    render(<AuthPanel />);
    await screen.findByRole('alert');
    await userEvent.click(screen.getByRole('button', { name: 'Обновить состояние входа' }));
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
    expect(await screen.findByText('Ely.by: реальный вход пока недоступен.')).toBeInTheDocument();
  });
});
