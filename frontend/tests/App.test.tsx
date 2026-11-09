import { invoke, isTauri } from '@tauri-apps/api/core';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { App } from '../src/app/App';
import authContract from '../../tests/contracts/auth.v1.json';
import { foundationSnapshot, workspaceFixture } from './fixtures';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

describe('foundation desktop screen', () => {
  beforeEach(() => vi.mocked(isTauri).mockReturnValue(true));

  it('enables game launch after connecting to the desktop backend', async () => {
    let resolve: ((value: unknown) => void) | undefined;
    const snapshot = new Promise((done) => {
      resolve = done;
    });
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'get_snapshot') return snapshot;
      if (command === 'get_workspace') return workspaceFixture;
      if (command === 'get_auth') return authContract;
      return null;
    });
    render(<App />);
    expect(screen.getByRole('status')).toHaveTextContent('Подключение к ядру');
    expect(screen.getByRole('button', { name: /Играть/ })).toBeDisabled();

    resolve?.(foundationSnapshot);
    expect(await screen.findByText('Ядро подключено')).toBeInTheDocument();
    expect(screen.getByText('macOS · ARM64')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('button', { name: /Играть/ })).toBeEnabled());
  });

  it('recovers from a backend failure on explicit retry', async () => {
    vi.mocked(invoke)
      .mockRejectedValueOnce('private backend details')
      .mockResolvedValueOnce(foundationSnapshot)
      .mockResolvedValueOnce(authContract)
      .mockResolvedValueOnce(workspaceFixture)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce(null);
    render(<App />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось подключиться');
    expect(screen.queryByText('private backend details')).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Повторить подключение' }));
    expect(await screen.findByText('Ядро подключено')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledTimes(6);
  });

  it('shows a clear browser-only state without a fake core', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    render(<App />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Откройте desktop-приложение');
    expect(invoke).not.toHaveBeenCalled();
    expect(screen.queryByText('Ядро подключено')).not.toBeInTheDocument();
  });
});
