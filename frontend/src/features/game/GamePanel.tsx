import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { getWorkspace } from '../profiles/client';

const labels = {
  preparing: 'Проверяем файлы игры и Java…',
  launching: 'Запускаем Minecraft…',
  running: 'Minecraft запущен',
  stopping: 'Останавливаем…',
  cancelled: 'Запуск отменён',
  finished: 'Игра завершена',
  failed: 'Не удалось запустить или продолжить игру',
};
type Stage = keyof typeof labels;
type GameStatus = {
  runId: string;
  sequence: number;
  profileName: string;
  stage: Stage;
  exit: { code: number | null; success: boolean; stopped: boolean } | null;
  errorCode: string | null;
  logSaved: boolean | null;
};

const errors: Record<string, string> = {
  authentication_required: 'Войдите в аккаунт, чтобы запустить игру.',
  auth_busy: 'Дождитесь завершения входа и попробуйте снова.',
  auth_rejected: 'Сессия Ely.by истекла. Войдите снова.',
  auth_network: 'Сервис Ely.by недоступен. Повторите попытку позже.',
  auth_unavailable: 'Этот способ входа недоступен в данной сборке.',
  game_busy: 'Дождитесь завершения текущего запуска или игры.',
  game_recovery_required:
    'После сбоя осталась запись о запуске. Закройте оставшееся окно Minecraft и выполните восстановление по документации.',
  storage_busy: 'Дождитесь завершения работы с файлами профиля.',
  revision_conflict: 'Профиль или сессия изменились. Нажмите «Играть» ещё раз.',
  invalid_settings: 'Проверьте настройки Java и памяти профиля.',
  invalid_launch_arguments: 'Параметры запуска Minecraft некорректны.',
  game_spawn: 'Не удалось запустить Java. Проверьте выбранную Java и настройки профиля.',
  game_monitor: 'Не удалось получить состояние процесса Minecraft.',
  network: 'Не удалось получить файлы Minecraft или Java. Проверьте подключение.',
  integrity: 'Не удалось подтвердить целостность файлов игры.',
  metadata: 'Не удалось прочитать метаданные Minecraft.',
  runtime: 'Для запуска нужна совместимая Java 21.',
  game_exited: 'Minecraft завершился с ошибкой.',
};

function message(code: string | null) {
  return code && Object.hasOwn(errors, code) ? errors[code] : 'Не удалось выполнить запуск.';
}

function parse(value: unknown): GameStatus | null {
  if (value === null) return null;
  if (!value || typeof value !== 'object') throw new Error('invalid_game_status');
  const v = value as Record<string, unknown>;
  if (
    v.schemaVersion !== 1 ||
    typeof v.runId !== 'string' ||
    typeof v.profileName !== 'string' ||
    typeof v.sequence !== 'number' ||
    !Number.isSafeInteger(v.sequence) ||
    v.sequence < 1 ||
    typeof v.stage !== 'string' ||
    !Object.hasOwn(labels, v.stage) ||
    (v.errorCode !== null && typeof v.errorCode !== 'string') ||
    (v.logSaved !== null && typeof v.logSaved !== 'boolean')
  )
    throw new Error('invalid_game_status');

  let exit: GameStatus['exit'] = null;
  if (v.exit !== null) {
    if (!v.exit || typeof v.exit !== 'object') throw new Error('invalid_game_exit');
    const result = v.exit as Record<string, unknown>;
    if (
      (result.code !== null &&
        (typeof result.code !== 'number' || !Number.isSafeInteger(result.code))) ||
      typeof result.stopped !== 'boolean' ||
      typeof result.success !== 'boolean'
    )
      throw new Error('invalid_game_exit');
    exit = { code: result.code, stopped: result.stopped, success: result.success };
  }
  return {
    runId: v.runId,
    sequence: v.sequence,
    profileName: v.profileName,
    stage: v.stage as Stage,
    exit,
    errorCode: v.errorCode,
    logSaved: v.logSaved,
  };
}

function errorMessage(error: unknown) {
  return message(
    error && typeof error === 'object' && 'code' in error && typeof error.code === 'string'
      ? error.code
      : null,
  );
}

export function GamePanel({ available }: { available: boolean }) {
  const [game, setGame] = useState<GameStatus | null>(null);
  const [connected, setConnected] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(false);
  const inFlight = useRef(false);

  function receive(value: unknown) {
    const next = parse(value);
    if (next) setGame((prev) => (prev && prev.sequence >= next.sequence ? prev : next));
  }

  useEffect(() => {
    if (!available) return;
    let active = true;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        unlisten = await listen<unknown>('launcher://game', ({ payload }) => {
          if (!active) return;
          try {
            receive(payload);
          } catch {
            setError('Не удалось прочитать статус игры.');
          }
        });
        if (!active) {
          unlisten();
          return;
        }
        const initial = await invoke<unknown>('get_game');
        if (active) {
          receive(initial);
          setConnected(true);
        }
      } catch {
        if (active) setError('Не удалось подключиться к управлению игрой.');
      }
    })();
    return () => {
      active = false;
      unlisten?.();
    };
  }, [available, attempt]);

  async function run(action: () => Promise<unknown>) {
    if (inFlight.current) return;
    inFlight.current = true;
    setPending(true);
    setError(undefined);
    try {
      receive(await action());
    } catch (e: unknown) {
      setError(errorMessage(e));
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }

  const running = game?.stage === 'running';
  const preparing = game?.stage === 'preparing' || game?.stage === 'launching';
  const stopping = game?.stage === 'stopping';
  return (
    <div className="play-area">
      <button
        className="button button--primary"
        type="button"
        disabled={!available || !connected || pending || running || preparing || stopping}
        aria-describedby="play-help"
        onClick={() => {
          void run(async () => {
            const workspace = await getWorkspace();
            const profile = workspace.profiles.find((p) => p.id === workspace.activeProfileId);
            if (!profile) throw new Error('missing_profile');
            return invoke('start_game', {
              request: {
                rootId: workspace.rootId,
                profileId: profile.id,
                profileRevision: profile.revision,
              },
            });
          });
        }}
      >
        <span aria-hidden="true">▶</span> Играть
      </button>
      {game && (running || preparing || stopping) && (
        <button
          className="button button--secondary"
          type="button"
          disabled={pending || stopping}
          onClick={() => {
            void run(() => invoke('stop_game', { request: { runId: game.runId } }));
          }}
        >
          {preparing ? 'Отменить запуск' : 'Принудительно остановить'}
        </button>
      )}
      {available && !connected && error && (
        <button
          className="button button--secondary"
          type="button"
          onClick={() => {
            setError(undefined);
            setAttempt((n) => n + 1);
          }}
        >
          Повторить подключение к игре
        </button>
      )}
      <p id="play-help" aria-live="polite">
        {!available
          ? 'Запуск доступен в подключённом приложении.'
          : game
            ? `${game.profileName} · ${game.exit?.stopped ? 'Игра остановлена' : labels[game.stage]}`
            : 'Лаунчер проверит файлы и запустит игру для активного профиля.'}
        {game?.exit?.code != null && !game.exit.stopped && ` · Код выхода ${game.exit.code}`}
      </p>
      {running && <p>Для сохранения мира выйдите через меню Minecraft.</p>}
      {(error || game?.errorCode) && (
        <p className="storage-error" role="alert">
          {error ?? message(game?.errorCode ?? null)}
        </p>
      )}
      {game?.logSaved === false && <p>Не удалось сохранить журнал последнего запуска.</p>}
    </div>
  );
}
