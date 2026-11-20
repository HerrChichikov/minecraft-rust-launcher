import { invoke } from '@tauri-apps/api/core';
import { listen, type Event } from '@tauri-apps/api/event';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthPanel } from '../src/features/auth/AuthPanel';
import { useAuth } from '../src/features/auth/useAuth';
import { getAuth, parseAuth } from '../src/features/auth/client';
import contract from '../../tests/contracts/auth.v2.json';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));
const production = parseAuth({
  ...contract,
  providers: [
    { ...contract.providers[0], available: true, flows: ['password'], unavailableReason: null },
  ],
});
const development = parseAuth({
  ...contract,
  providers: [
    ...production.providers,
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
function AuthHarness() {
  const model = useAuth();
  return <AuthPanel model={model} />;
}
describe('authentication UI and protocol', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(listen).mockClear();
  });
  it('does not manufacture production login or offer mock without backend capability', async () => {
    vi.mocked(invoke).mockResolvedValue(production);
    render(<AuthHarness />);
    expect(await screen.findByLabelText('Пароль')).toBeInTheDocument();
    expect(screen.queryByLabelText('Тестовый ник')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Выйти' })).not.toBeInTheDocument();
  });
  it('uses backend-owned identity, reconnects and logs out by revision', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(development)
      .mockResolvedValueOnce(authenticated)
      .mockResolvedValueOnce(authenticated)
      .mockResolvedValueOnce({ ...production, providers: development.providers, revision: 2 });
    const view = render(<AuthHarness />);
    await userEvent.click(await screen.findByRole('button', { name: 'Тестовый вход' }));
    const input = screen.getByLabelText('Тестовый ник');
    await userEvent.type(input, 'Notch');
    await userEvent.click(screen.getByRole('button', { name: 'Войти в Development / Mock' }));
    expect(await screen.findByText('Notch')).toBeInTheDocument();
    expect(screen.getByText(/Development \/ Mock доступен/)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('login', {
      request: {
        expectedRevision: 0,
        providerId: 'development-mock',
        input: { flow: 'development_nickname', nickname: 'Notch' },
      },
    });
    view.unmount();
    render(<AuthHarness />);
    expect(await screen.findByText('Notch')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Выйти' }));
    expect(await screen.findByLabelText('Пароль')).toBeInTheDocument();
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
    render(<AuthHarness />);
    await userEvent.click(await screen.findByRole('button', { name: 'Тестовый вход' }));
    await userEvent.type(screen.getByLabelText('Тестовый ник'), 'Notch');
    await userEvent.click(screen.getByRole('button', { name: 'Войти в Development / Mock' }));
    expect(screen.getByRole('button', { name: 'Войти в Development / Mock' })).toBeDisabled();
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
      parseAuth({
        ...production,
        providers: [{ ...production.providers[0], flows: ['development_nickname'] }],
      }),
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
    render(<AuthHarness />);
    await userEvent.click(screen.getByRole('button', { name: 'Обновить состояние входа' }));
    expect(await screen.findByText('Notch')).toBeInTheDocument();
    await act(async () => {
      resolve?.(development);
      await Promise.resolve();
    });
    expect(screen.getByText('Notch')).toBeInTheDocument();
    expect(screen.queryByLabelText('Тестовый ник')).not.toBeInTheDocument();
  });
  it('updates the account when a launch invalidates the session outside the auth form', async () => {
    let publish: ((event: Event<unknown>) => void) | undefined;
    vi.mocked(listen).mockImplementationOnce((event, handler) => {
      expect(event).toBe('launcher://auth');
      publish = handler;
      return Promise.resolve(() => {});
    });
    vi.mocked(invoke).mockResolvedValueOnce(authenticated);
    render(<AuthHarness />);
    expect(await screen.findByText('Notch')).toBeInTheDocument();
    act(() =>
      publish?.({ event: 'launcher://auth', id: 1, payload: { ...production, revision: 2 } }),
    );
    expect(screen.queryByText('Notch')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Пароль')).toBeInTheDocument();
  });
  it('maps unknown backend errors without exposing their text', async () => {
    vi.mocked(invoke).mockRejectedValue({ code: 'unexpected', message: 'private-token' });
    await expect(getAuth()).rejects.toMatchObject({ code: 'auth_failed' });
  });
  it('lets the user recover a failed initial snapshot', async () => {
    vi.mocked(invoke).mockRejectedValueOnce('private').mockResolvedValueOnce(production);
    render(<AuthHarness />);
    await screen.findByRole('alert');
    await userEvent.click(screen.getByRole('button', { name: 'Обновить состояние входа' }));
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
    expect(await screen.findByLabelText('Пароль')).toBeInTheDocument();
  });
});

describe('platform account and game profiles', () => {
  const firstProfile = {
    id: '12345678-1234-4234-8123-123456789abc',
    realm: 'survival',
    nickname: 'AliceMC',
  };
  const realAccount = parseAuth({
    ...production,
    revision: 2,
    account: {
      providerId: 'platform',
      playerId: 'b50ad385-829d-3141-a216-7e7d7539ba7f',
      playerName: 'Alice',
      development: false,
    },
    gameProfiles: [firstProfile],
  });
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });
  it('registers with an invite, selects an owned game profile and reports an unconfirmed remote logout', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(production)
      .mockResolvedValueOnce({ ...production, revision: 1, notice: 'registered' })
      .mockResolvedValueOnce(realAccount)
      .mockResolvedValueOnce({
        ...realAccount,
        revision: 3,
        selectedGameProfileId: firstProfile.id,
      })
      .mockResolvedValueOnce({ ...production, revision: 4, notice: 'remote_logout_failed' });
    render(<AuthHarness />);
    await userEvent.click(await screen.findByRole('button', { name: 'Регистрация' }));
    await userEvent.type(screen.getByLabelText('Логин'), 'Alice');
    await userEvent.type(screen.getByLabelText('Пароль'), 'fixture-password');
    await userEvent.type(screen.getByLabelText('Код приглашения'), 'INVITE1');
    await userEvent.click(screen.getByRole('button', { name: 'Создать аккаунт' }));
    expect(await screen.findByText(/Аккаунт создан/)).toBeInTheDocument();
    expect(screen.getByLabelText('Пароль')).toHaveValue('');
    expect(invoke).toHaveBeenCalledWith('register_account', {
      request: {
        expectedRevision: 0,
        nickname: 'Alice',
        password: 'fixture-password',
        email: '',
        invitationCode: 'INVITE1',
      },
    });
    await userEvent.type(screen.getByLabelText('Пароль'), 'fixture-password');
    await userEvent.click(screen.getByRole('button', { name: 'Войти' }));
    expect(await screen.findByText('Alice')).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByLabelText('Игровой профиль'), firstProfile.id);
    expect(invoke).toHaveBeenCalledWith('select_game_profile', {
      request: { expectedRevision: 2, gameProfileId: firstProfile.id },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Выйти' }));
    expect(
      await screen.findByText(/Сервер не подтвердил отзыв удалённой сессии/),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Пароль')).toHaveValue('');
  });
  it('clears the password on network failure and never echoes provider error text', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(production)
      .mockRejectedValueOnce({ code: 'auth_network', message: 'fixture-password-secret' })
      .mockResolvedValueOnce(production);
    render(<AuthHarness />);
    await userEvent.type(await screen.findByLabelText('Логин или email'), 'fixture@example.test');
    await userEvent.type(screen.getByLabelText('Пароль'), 'fixture-password-secret');
    await userEvent.click(screen.getByRole('button', { name: 'Войти' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Не удалось связаться с сервером авторизации',
    );
    expect(screen.getByLabelText('Пароль')).toHaveValue('');
    expect(screen.queryByText('fixture-password-secret')).not.toBeInTheDocument();
  });
  it('accepts short and Unicode account nicknames preserved by the migration', () => {
    for (const playerName of ['ab', 'Я'.repeat(40)]) {
      expect(
        parseAuth({ ...realAccount, account: { ...realAccount.account, playerName } }).account
          ?.playerName,
      ).toBe(playerName);
    }
  });
  it('rejects a selection missing from the authoritative list of profiles', () => {
    expect(() =>
      parseAuth({ ...realAccount, gameProfiles: [], selectedGameProfileId: firstProfile.id }),
    ).toThrow();
    expect(() => parseAuth({ ...production, gameProfiles: [firstProfile] })).toThrow();
  });
});

describe('secure credential storage UI', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });
  const supported = { ...production, rememberSupported: true };
  const restored = {
    ...supported,
    revision: 1,
    persistence: 'system_credential_store',
    account: {
      providerId: 'platform',
      playerId: '12345678-1234-4234-8123-123456789abc',
      playerName: 'Alice',
      development: false,
    },
  };
  it('requires explicit remember intent and only reports persistence confirmed by backend', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(supported).mockResolvedValueOnce(restored);
    render(<AuthHarness />);
    const checkbox = await screen.findByRole('checkbox');
    expect(checkbox).not.toBeChecked();
    await userEvent.type(screen.getByLabelText('Логин или email'), 'fixture@example.test');
    await userEvent.type(screen.getByLabelText('Пароль'), 'fixture-password');
    await userEvent.click(checkbox);
    await userEvent.click(screen.getByRole('button', { name: 'Войти' }));
    expect(await screen.findByText('Alice')).toBeInTheDocument();
    expect(screen.getByText(/Сессия сохранена в системном/)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('login', {
      request: {
        expectedRevision: 0,
        providerId: 'platform',
        remember: true,
        input: { flow: 'password', login: 'fixture@example.test', password: 'fixture-password' },
      },
    });
  });
  it('restores only on an explicit click, and shows memory-only fallback', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(supported)
      .mockResolvedValueOnce({
        ...restored,
        persistence: 'memory_only',
        notice: 'credential_store_unavailable',
      });
    render(<AuthHarness />);
    await screen.findByRole('checkbox');
    expect(invoke).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole('button', { name: 'Восстановить сохранённую сессию' }));
    expect(await screen.findByText(/Не удалось сохранить сессию/)).toBeInTheDocument();
    expect(screen.queryByText(/Сессия сохранена в системном/)).not.toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('restore_auth', { request: { expectedRevision: 0 } });
  });
  it('never accepts persistent mock or a persistence claim without store capability', () => {
    expect(() =>
      parseAuth({
        ...authenticated,
        persistence: 'system_credential_store',
        rememberSupported: true,
      }),
    ).toThrow();
    expect(() => parseAuth({ ...restored, rememberSupported: false })).toThrow();
  });
});
