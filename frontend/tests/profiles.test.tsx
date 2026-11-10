import { invoke, isTauri } from '@tauri-apps/api/core';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ProfilesPanel } from '../src/features/profiles/ProfilesPanel';
import { useWorkspace } from '../src/features/profiles/useWorkspace';
import { getWorkspace, parseWorkspace, StorageError } from '../src/features/profiles/client';
import { workspaceFixture as fixture } from './fixtures';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

function ProfilesHarness() {
  const model = useWorkspace();
  return <ProfilesPanel model={model} operationActive={false} />;
}
describe('profiles and persistent settings', () => {
  beforeEach(() => vi.mocked(isTauri).mockReturnValue(true));

  it('keeps the browser disconnected from storage', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(getWorkspace()).rejects.toMatchObject({ code: 'desktop_required' });
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([
    { ...fixture, schemaVersion: 2 },
    { ...fixture, activeProfileId: 'unknown' },
    { ...fixture, profiles: [{ ...fixture.profiles[0], id: '../../outside' }] },
    { ...fixture, profiles: [...fixture.profiles, ...fixture.profiles] },
    {
      ...fixture,
      profiles: [{ ...fixture.profiles[0], settings: { minRamMib: 4096, maxRamMib: 1024 } }],
    },
  ])('rejects malformed workspace responses', (value) => {
    expect(() => parseWorkspace(value)).toThrow(StorageError);
  });

  it('loads the actual profile before enabling settings', async () => {
    let resolve: ((value: unknown) => void) | undefined;
    vi.mocked(invoke).mockReturnValue(
      new Promise((done) => {
        resolve = done;
      }),
    );
    render(<ProfilesHarness />);
    expect(screen.getByRole('status')).toHaveTextContent('Загрузка профилей');
    expect(screen.queryByLabelText('Название')).not.toBeInTheDocument();
    resolve?.(fixture);
    expect(await screen.findByLabelText('Название')).toHaveValue('Основная конфигурация');
    expect(screen.getByRole('spinbutton', { name: 'Максимум RAM, МиБ' })).toHaveValue(4096);
  });

  it('sends validated revision context and waits for a successful save', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(fixture);
    render(<ProfilesHarness />);
    const name = await screen.findByLabelText('Название');
    await userEvent.clear(name);
    await userEvent.type(name, 'Изменённый мир');
    const max = screen.getByRole('spinbutton', { name: 'Максимум RAM, МиБ' });
    await userEvent.clear(max);
    await userEvent.type(max, '2048');
    let resolve: ((value: unknown) => void) | undefined;
    vi.mocked(invoke).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done;
      }),
    );
    await userEvent.click(screen.getByRole('button', { name: 'Сохранить настройки' }));
    expect(screen.getByRole('button', { name: 'Сохранить настройки' })).toBeDisabled();
    expect(screen.queryByText('Настройки сохранены')).not.toBeInTheDocument();
    expect(invoke).toHaveBeenLastCalledWith('save_profile', {
      request: {
        rootId: fixture.rootId,
        profileId: fixture.activeProfileId,
        expectedRevision: 1,
        name: 'Изменённый мир',
        settings: { ...fixture.profiles[0]?.settings, maxRamMib: 2048 },
      },
    });
    resolve?.({
      ...fixture,
      profiles: [
        {
          ...fixture.profiles[0],
          revision: 2,
          name: 'Изменённый мир',
          settings: { ...fixture.profiles[0]?.settings, maxRamMib: 2048 },
        },
      ],
    });
    expect(await screen.findByText('Настройки сохранены')).toBeInTheDocument();
    expect(screen.getByLabelText('Название')).toHaveValue('Изменённый мир');
  });

  it('preserves the draft on failure and only displays sanitized backend errors', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(fixture)
      .mockRejectedValueOnce({ code: 'revision_conflict', message: '/private/token-secret' });
    render(<ProfilesHarness />);
    const name = await screen.findByLabelText('Название');
    await userEvent.clear(name);
    await userEvent.type(name, 'Unsaved draft');
    await userEvent.click(screen.getByRole('button', { name: 'Сохранить настройки' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Настройки уже изменились');
    expect(name).toHaveValue('Unsaved draft');
    expect(screen.queryByText(/token-secret/)).not.toBeInTheDocument();
    expect(screen.queryByText('Настройки сохранены')).not.toBeInTheDocument();
    vi.mocked(invoke).mockResolvedValueOnce({
      ...fixture,
      profiles: [{ ...fixture.profiles[0], revision: 2, name: 'Saved elsewhere' }],
    });
    await userEvent.click(screen.getByRole('button', { name: 'Обновить список' }));
    await waitFor(() => expect(screen.getByLabelText('Название')).toHaveValue('Saved elsewhere'));
  });

  it('creates and selects profiles without inventing an active profile', async () => {
    const secondId = '34567890-1234-4567-8123-123456789abc';
    const second = { ...fixture.profiles[0], id: secondId, name: 'Новый мир' };
    const two = { ...fixture, profiles: [...fixture.profiles, second] };
    vi.mocked(invoke)
      .mockResolvedValueOnce(fixture)
      .mockResolvedValueOnce(two)
      .mockResolvedValueOnce({ ...two, activeProfileId: secondId, revision: 2 });
    render(<ProfilesHarness />);
    const input = await screen.findByLabelText('Название нового профиля');
    await userEvent.type(input, 'Новый мир');
    await userEvent.click(screen.getByRole('button', { name: 'Создать профиль' }));
    await screen.findByRole('option', { name: 'Новый мир' });
    expect(screen.getByLabelText('Активный профиль')).toHaveValue(fixture.activeProfileId);
    await userEvent.selectOptions(screen.getByLabelText('Активный профиль'), secondId);
    await waitFor(() => expect(screen.getByLabelText('Название')).toHaveValue('Новый мир'));
    expect(invoke).toHaveBeenLastCalledWith('select_profile', {
      request: { rootId: fixture.rootId, profileId: secondId, expectedRevision: 1 },
    });
  });

  it('explains no migration before switching the root and uses configuration revision', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(fixture)
      .mockResolvedValueOnce({
        ...fixture,
        dataRoot: '/new/data',
        rootId: '45678901-1234-4567-8123-123456789abc',
        configurationRevision: 2,
      });
    render(<ProfilesHarness />);
    const path = await screen.findByLabelText('Новый путь');
    expect(screen.getByText(/прежние миры и настройки останутся/)).toBeInTheDocument();
    await userEvent.clear(path);
    await userEvent.type(path, '/new/data');
    await userEvent.click(screen.getByRole('button', { name: 'Сменить папку' }));
    await waitFor(() => expect(screen.getByText('/new/data')).toBeInTheDocument());
    expect(invoke).toHaveBeenLastCalledWith('switch_data_root', {
      request: { rootId: fixture.rootId, expectedRevision: 1, path: '/new/data' },
    });
  });
});
