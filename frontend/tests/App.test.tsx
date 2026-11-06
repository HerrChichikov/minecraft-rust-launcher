import { invoke, isTauri } from '@tauri-apps/api/core';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { App } from '../src/app/App';
import { foundationSnapshot, workspaceFixture } from './fixtures';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

describe('foundation desktop screen', () => {
  beforeEach(() => vi.mocked(isTauri).mockReturnValue(true));

  it('renders a real loading state and keeps game launch unavailable', async () => {
    let resolve: ((value: unknown) => void) | undefined;
    vi.mocked(invoke).mockReturnValue(
      new Promise((done) => {
        resolve = done;
      }),
    );
    render(<App />);
    expect(screen.getByRole('status')).toHaveTextContent('Подключение к ядру');
    expect(screen.getByRole('button', { name: /Играть/ })).toBeDisabled();

    resolve?.(foundationSnapshot);
    expect(await screen.findByText('Ядро подключено')).toBeInTheDocument();
    expect(screen.getByText('macOS · ARM64')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Играть/ })).toBeDisabled();
  });

  it('recovers from a backend failure on explicit retry', async () => {
    vi.mocked(invoke)
      .mockRejectedValueOnce('private backend details')
      .mockResolvedValueOnce(foundationSnapshot)
      .mockResolvedValueOnce(workspaceFixture)
      .mockResolvedValueOnce(null);
    render(<App />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Не удалось подключиться');
    expect(screen.queryByText('private backend details')).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Повторить подключение' }));
    expect(await screen.findByText('Ядро подключено')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledTimes(4);
  });

  it('shows a clear browser-only state without a fake core', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    render(<App />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Откройте desktop-приложение');
    expect(invoke).not.toHaveBeenCalled();
    expect(screen.queryByText('Ядро подключено')).not.toBeInTheDocument();
  });
});
