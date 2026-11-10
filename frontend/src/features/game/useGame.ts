import { useOperation } from '../../shared/ipc/useOperation';
import { parseProgress, type Progress } from '../profiles/installationProtocol';

export const labels = {
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
  profileId: string;
  rootId: string;
  preparation: Progress | null;
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

export function message(code: string | null) {
  return (
    (code && Object.hasOwn(errors, code) ? errors[code] : undefined) ??
    'Не удалось выполнить запуск.'
  );
}

function parse(value: unknown): GameStatus | null {
  if (value === null) return null;
  if (!value || typeof value !== 'object') throw new Error('invalid_game_status');
  const v = value as Record<string, unknown>;
  if (
    v.schemaVersion !== 1 ||
    typeof v.runId !== 'string' ||
    typeof v.profileName !== 'string' ||
    typeof v.profileId !== 'string' ||
    typeof v.rootId !== 'string' ||
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
    profileId: v.profileId,
    rootId: v.rootId,
    preparation: v.preparation === null ? null : parseProgress(v.preparation),
    stage: v.stage as Stage,
    exit,
    errorCode: v.errorCode,
    logSaved: v.logSaved,
  };
}

export function errorMessage(error: unknown) {
  return message(
    error && typeof error === 'object' && 'code' in error && typeof error.code === 'string'
      ? error.code
      : null,
  );
}

export function useGame() {
  const channel = useOperation({
    event: 'launcher://game',
    snapshot: 'get_game',
    parse,
    connectionError: 'Не удалось подключиться к управлению игрой.',
  });
  const stage = channel.status?.stage;
  const active =
    channel.pending ||
    stage === 'running' ||
    stage === 'preparing' ||
    stage === 'launching' ||
    stage === 'stopping';
  return { ...channel, active };
}
