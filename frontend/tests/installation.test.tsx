import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Event } from '@tauri-apps/api/event';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { InstallationPanel } from '../src/features/profiles/InstallationPanel';
import { parseInstallation } from '../src/features/profiles/installationProtocol';
import { workspaceFixture } from './fixtures';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));
const running = {
  schemaVersion: 1,
  operationId: '12345678-1234-4567-8123-123456789abc',
  profileId: workspaceFixture.activeProfileId,
  profileName: 'Основная конфигурация',
  stage: 'downloading',
  sequence: 1,
  completedBytes: 10,
  totalBytes: 100,
  errorCode: null,
  terminal: false,
};
let callback: ((event: Event<unknown>) => void) | undefined;
function emit(value: unknown) {
  act(() => callback?.({ event: 'launcher://installation', id: 1, payload: value }));
}
describe('installation UI', () => {
  beforeEach(() => {
    callback = undefined;
    vi.mocked(listen).mockImplementation((_name, handler) => {
      callback = handler;
      return Promise.resolve(() => {});
    });
  });
  it('reconnects to real backend state without manufacturing success', async () => {
    vi.mocked(invoke).mockResolvedValue(running);
    render(<InstallationPanel />);
    expect(await screen.findByText('Проверка и загрузка файлов')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Проверить и установить' })).toBeDisabled();
    expect(screen.queryByText('Minecraft и Java подготовлены')).not.toBeInTheDocument();
  });
  it('does not roll a terminal event back to an older start response', async () => {
    let resolve: ((value: unknown) => void) | undefined;
    vi.mocked(invoke).mockImplementation((command) => {
      if (command === 'get_installation') return Promise.resolve(null);
      if (command === 'get_workspace') return Promise.resolve(workspaceFixture);
      return new Promise((done) => {
        resolve = done;
      });
    });
    render(<InstallationPanel />);
    const start = screen.getByRole('button', { name: 'Проверить и установить' });
    await waitFor(() => expect(start).toBeEnabled());
    await userEvent.click(start);
    emit({ ...running, sequence: 3, stage: 'ready', terminal: true });
    resolve?.({ ...running, sequence: 0, stage: 'initializing' });
    expect(await screen.findByText('Minecraft и Java подготовлены')).toBeInTheDocument();
    await waitFor(() => expect(start).toBeEnabled());
    expect(invoke).toHaveBeenCalledWith('start_install', {
      request: { rootId: workspaceFixture.rootId, profileId: workspaceFixture.activeProfileId },
    });
  });
  it('cancels only the operation identified by the backend', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(running).mockResolvedValueOnce(undefined);
    render(<InstallationPanel />);
    const cancel = await screen.findByRole('button', { name: 'Отменить' });
    await userEvent.click(cancel);
    expect(invoke).toHaveBeenLastCalledWith('cancel_install', {
      request: { operationId: running.operationId },
    });
    emit({ ...running, sequence: 2, stage: 'cancelled', terminal: true, errorCode: 'cancelled' });
    expect(screen.getByText('Установка отменена')).toBeInTheDocument();
  });
  it('displays sanitized failure and offers retry', async () => {
    vi.mocked(invoke).mockResolvedValue({
      ...running,
      stage: 'error',
      sequence: 2,
      terminal: true,
      errorCode: 'network',
    });
    render(<InstallationPanel />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Проверьте подключение');
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Проверить и установить' })).toBeEnabled(),
    );
  });
  it.each([undefined, {}, { ...running, schemaVersion: 99 }, { ...running, completedBytes: '10' }])(
    'rejects malformed backend payloads',
    (value) => {
      expect(() => parseInstallation(value)).toThrow();
    },
  );
});
