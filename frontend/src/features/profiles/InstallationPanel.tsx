import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getWorkspace } from './client';

import { parseInstallation } from './installationProtocol';
import type { Status } from './installationProtocol';

const stages: Record<string, string> = {
  initializing: 'Получение метаданных',
  checking: 'Проверка файлов',
  downloading: 'Проверка и загрузка файлов',
  extracting: 'Подготовка Java',
  ready: 'Minecraft и Java подготовлены',
  cancelling: 'Отмена операции…',
  cancelled: 'Установка отменена',
  error: 'Установка не завершена',
};
const errors: Record<string, string> = {
  network: 'Не удалось загрузить файл. Проверьте подключение и повторите установку.',
  integrity: 'Скачанный файл не прошёл проверку. Повторите установку.',
  metadata: 'Метаданные имеют неподдерживаемый формат или адрес. Обновите приложение.',
  runtime: 'Java не прошла проверку версии или архитектуры.',
  storage: 'Проверьте права доступа, свободное место и занятость хранилища.',
};
export function InstallationPanel() {
  const [status, setStatus] = useState<Status | null>(null);
  const [loading, setLoading] = useState(true);
  const [starting, setStarting] = useState(false);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string>();
  const alive = useRef(false);
  function update(next: Status | null) {
    setStatus((previous) =>
      previous &&
      next &&
      previous.operationId === next.operationId &&
      previous.sequence > next.sequence
        ? previous
        : next,
    );
  }
  useEffect(() => {
    let active = true;
    let cleanup: (() => void) | undefined;
    let received = false;
    alive.current = true;
    void listen<unknown>('launcher://installation', (event) => {
      if (active) {
        try {
          const next = parseInstallation(event.payload);
          received = true;
          update(next);
        } catch {
          setError('Ответ ядра имеет неподдерживаемый формат.');
        }
      }
    }).then(
      async (unlisten) => {
        if (!active) {
          unlisten();
          return;
        }
        cleanup = unlisten;
        setConnected(true);
        try {
          const next = parseInstallation(await invoke<unknown>('get_installation'));
          if (active && !received) update(next);
        } catch {
          if (active) setError('Не удалось получить состояние установки.');
        } finally {
          if (active) setLoading(false);
        }
      },
      () => {
        if (active) {
          setError('Не удалось подключиться к событиям установки.');
          setLoading(false);
        }
      },
    );
    return () => {
      active = false;
      alive.current = false;
      cleanup?.();
    };
  }, []);
  const busy = starting || (status !== null && !status.terminal);
  async function start() {
    setStarting(true);
    setError(undefined);
    try {
      const workspace = await getWorkspace();
      const next = parseInstallation(
        await invoke<unknown>('start_install', {
          request: { rootId: workspace.rootId, profileId: workspace.activeProfileId },
        }),
      );
      if (alive.current) update(next);
    } catch {
      if (alive.current)
        setError(
          'Не удалось начать установку. Проверьте хранилище и дождитесь завершения другой операции.',
        );
    } finally {
      if (alive.current) setStarting(false);
    }
  }
  return (
    <section className="profiles-card" aria-labelledby="installation-title" aria-busy={busy}>
      <p className="eyebrow">Minecraft 1.21.1</p>
      <h2 id="installation-title">Установка игры</h2>
      <p className="muted">
        Официальные файлы клиента, библиотек, ресурсов и Java 21. При первом запуске потребуется
        около 1–2 ГБ свободного места и подключение к сети.
      </p>
      {loading && <p role="status">Подключение к установщику…</p>}
      {status && (
        <div aria-live="polite">
          <p className="field-help">Последняя операция · {status.profileName}</p>
          <p>{stages[status.stage] ?? 'Операция выполняется'}</p>
          {!status.terminal && status.totalBytes !== null && status.totalBytes > 0 && (
            <>
              <progress
                max={status.totalBytes}
                value={Math.min(status.completedBytes, status.totalBytes)}
                aria-label="Подготовленные файлы"
              />
              <p className="field-help">
                Подготовлено {Math.floor(status.completedBytes / 1048576)} /{' '}
                {Math.ceil(status.totalBytes / 1048576)} МиБ. Уже проверенные файлы используются
                повторно.
              </p>
            </>
          )}
          {status.errorCode && status.errorCode !== 'cancelled' && (
            <p role="alert">{errors[status.errorCode] ?? errors.storage}</p>
          )}
        </div>
      )}
      {error && (
        <p role="alert" className="storage-error">
          {error}
        </p>
      )}
      <div className="installation-actions">
        <button
          className="button button--primary"
          disabled={loading || busy || !connected}
          type="button"
          onClick={() => {
            void start();
          }}
        >
          Проверить и установить
        </button>
        {busy && status && (
          <button
            className="button button--secondary"
            type="button"
            disabled={status.stage === 'cancelling'}
            onClick={() => {
              void invoke('cancel_install', { request: { operationId: status.operationId } }).catch(
                () => setError('Не удалось отправить запрос отмены.'),
              );
            }}
          >
            Отменить
          </button>
        )}
      </div>
      <p className="field-help">
        Проверка пересчитывает SHA1 файлов. Повреждённые файлы восстанавливаются; игровые миры
        сохраняются. Вход в аккаунт доступен в панели выше. Для запуска нажмите «Играть».
      </p>
    </section>
  );
}
