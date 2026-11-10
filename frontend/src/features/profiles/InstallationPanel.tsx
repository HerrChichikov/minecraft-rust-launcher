import { invoke } from '@tauri-apps/api/core';
import type { Workspace } from './client';
import { OperationProgress } from './OperationProgress';
import type { useInstallation } from './useInstallation';

const errors: Record<string, string> = {
  network: 'Не удалось загрузить файл. Проверьте подключение и повторите установку.',
  integrity: 'Скачанный файл не прошёл проверку. Повторите установку.',
  metadata: 'Формат данных Minecraft изменился. Обновите лаунчер.',
  runtime: 'Для запуска нужна совместимая Java 21. Проверьте её в настройках.',
  storage: 'Проверьте права доступа и свободное место в папке данных.',
  storage_busy: 'Файлы заняты другой операцией. Дождитесь её завершения.',
  revision_conflict: 'Папка данных изменилась. Обновите профиль в настройках.',
};

function failure(value: unknown) {
  const code =
    value && typeof value === 'object' && 'code' in value && typeof value.code === 'string'
      ? value.code
      : '';
  return (
    (Object.hasOwn(errors, code) ? errors[code] : undefined) ??
    'Не удалось начать установку. Проверьте папку данных и попробуйте снова.'
  );
}

export function InstallationPanel({
  model,
  workspace,
  gameActive,
  prepared,
}: {
  model: ReturnType<typeof useInstallation>;
  workspace: Workspace | undefined;
  gameActive: boolean;
  prepared: boolean;
}) {
  const { status, active, pending, connected, connectionError, error, reconnect, execute } = model;
  const current =
    status &&
    (!status.terminal ||
      (status.rootId === workspace?.rootId && status.profileId === workspace.activeProfileId))
      ? status
      : null;
  const canStart = workspace && connected && !active && !gameActive;
  return (
    <section
      className="panel installation-card"
      aria-labelledby="installation-title"
      aria-busy={active}
    >
      <p className="eyebrow">Файлы игры</p>
      <h2 id="installation-title">Установка</h2>
      {current ? (
        <>
          {!current.terminal && <p className="field-help">{current.profileName}</p>}
          <OperationProgress progress={current} active={!current.terminal} />
          {current.stage === 'error' && (
            <p role="alert" className="storage-error">
              {current.errorCode && Object.hasOwn(errors, current.errorCode)
                ? errors[current.errorCode]
                : errors.storage}
            </p>
          )}
        </>
      ) : (
        <p className="muted">
          {pending
            ? 'Начинаем проверку…'
            : !connected && !connectionError
              ? 'Подключаемся к установщику…'
              : prepared
                ? 'Файлы ранее подготовлены. Лаунчер проверит их перед игрой.'
                : 'Файлы ещё не проверены для этого профиля.'}
        </p>
      )}
      {(connectionError || error) && (
        <p role="alert" className="storage-error">
          {connectionError ?? error}
        </p>
      )}
      <div className="button-row">
        <button
          className="button button--secondary"
          disabled={!canStart}
          type="button"
          onClick={() => {
            if (canStart)
              void execute(
                () =>
                  invoke('start_install', {
                    request: { rootId: workspace.rootId, profileId: workspace.activeProfileId },
                  }),
                failure,
              );
          }}
        >
          Проверить и установить
        </button>
        {current && !current.terminal && (
          <button
            className="button button--quiet"
            type="button"
            disabled={pending || !connected || current.stage === 'cancelling'}
            onClick={() => {
              void execute(
                () => invoke('cancel_install', { request: { operationId: current.operationId } }),
                () => 'Не удалось отменить установку. Повторите попытку.',
                false,
              );
            }}
          >
            Отменить
          </button>
        )}
        {connectionError && (
          <button className="button button--quiet" type="button" onClick={reconnect}>
            Подключиться снова
          </button>
        )}
      </div>
    </section>
  );
}
