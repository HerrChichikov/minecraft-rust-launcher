import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import type { Workspace } from '../profiles/client';
import { OperationProgress } from '../profiles/OperationProgress';

import { errorMessage, labels, message, type useGame } from './useGame';

export function GamePanel({
  model,
  workspace,
  available,
  installActive,
  controlsBusy = false,
  signedIn,
  onSignIn,
}: {
  model: ReturnType<typeof useGame>;
  workspace: Workspace | undefined;
  available: boolean;
  installActive: boolean;
  controlsBusy?: boolean;
  signedIn: boolean;
  onSignIn: () => void;
}) {
  const { status, connected, pending, active, connectionError, error, execute, reconnect } = model;
  const [confirmRun, setConfirmRun] = useState<string>();
  const game =
    status &&
    (active ||
      (status.rootId === workspace?.rootId && status.profileId === workspace.activeProfileId))
      ? status
      : null;
  const running = game?.stage === 'running';
  const preparing = game?.stage === 'preparing' || game?.stage === 'launching';
  const stopping = game?.stage === 'stopping';
  const profile = workspace?.profiles.find((item) => item.id === workspace.activeProfileId);
  const confirm = running && confirmRun === game.runId;

  function stop(runId: string) {
    setConfirmRun(undefined);
    void execute(() => invoke('stop_game', { request: { runId } }), errorMessage);
  }

  return (
    <div className="play-area">
      <div className="button-row">
        <button
          className="button button--primary play-button"
          type="button"
          disabled={
            !available ||
            !connected ||
            !workspace ||
            !profile ||
            active ||
            installActive ||
            controlsBusy
          }
          aria-describedby="play-help"
          onClick={() => {
            if (!workspace || !profile) return;
            if (!signedIn) {
              onSignIn();
              return;
            }
            void execute(
              () =>
                invoke('start_game', {
                  request: {
                    rootId: workspace.rootId,
                    profileId: profile.id,
                    profileRevision: profile.revision,
                  },
                }),
              errorMessage,
            );
          }}
        >
          <svg aria-hidden="true" viewBox="0 0 24 24">
            <path d="m8 5 12 7-12 7z" fill="currentColor" />
          </svg>
          {active ? 'Играть' : signedIn ? 'Играть' : 'Войти для игры'}
        </button>
        {game && (running || preparing || stopping) && (
          <button
            className="button button--secondary"
            type="button"
            disabled={!connected || pending || stopping}
            aria-expanded={running ? confirm : undefined}
            onClick={() => {
              if (running) setConfirmRun(confirm ? undefined : game.runId);
              else stop(game.runId);
            }}
          >
            {preparing ? 'Отменить запуск' : stopping ? 'Останавливаем…' : 'Остановить игру'}
          </button>
        )}
        {connectionError && (
          <button className="button button--secondary" type="button" onClick={reconnect}>
            Подключиться снова
          </button>
        )}
      </div>
      {confirm && (
        <div className="stop-confirm">
          <p>
            Для сохранения мира выйдите через меню Minecraft. При принудительной остановке последние
            изменения могут потеряться.
          </p>
          <div className="button-row">
            <button
              className="button button--danger"
              type="button"
              onClick={() => stop(game.runId)}
            >
              Принудительно остановить
            </button>
            <button
              className="button button--quiet"
              type="button"
              onClick={() => setConfirmRun(undefined)}
            >
              Назад
            </button>
          </div>
        </div>
      )}
      <p id="play-help" role="status" className="muted">
        {!available
          ? 'Запуск доступен в desktop-приложении.'
          : pending && !game
            ? 'Отправляем запрос…'
            : installActive
              ? 'Дождитесь завершения установки.'
              : controlsBusy && !game
                ? 'Дождитесь завершения текущего действия.'
                : game
                  ? `${game.profileName} · ${game.exit?.stopped ? 'Игра остановлена' : labels[game.stage]}`
                  : !connected && !connectionError
                    ? 'Подключаемся к игре…'
                    : !signedIn
                      ? 'Войдите в аккаунт, чтобы играть.'
                      : 'Лаунчер проверит файлы перед запуском.'}
        {game?.exit?.code != null && !game.exit.stopped && ` · Код выхода ${game.exit.code}`}
      </p>
      {game?.stage === 'preparing' && game.preparation && (
        <OperationProgress
          progress={game.preparation}
          active
          label={game.preparation.stage === 'ready' ? 'Готовим запуск Minecraft' : undefined}
        />
      )}
      {(connectionError || error || game?.errorCode) && (
        <p className="storage-error" role="alert">
          {connectionError ?? error ?? message(game?.errorCode ?? null)}
        </p>
      )}
      {game?.logSaved === false && (
        <p className="field-help">Не удалось сохранить журнал последнего запуска.</p>
      )}
    </div>
  );
}
