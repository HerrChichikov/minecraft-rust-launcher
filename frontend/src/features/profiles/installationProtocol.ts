export const stageLabels = {
  initializing: 'Получаем метаданные',
  checking: 'Проверяем файлы',
  downloading: 'Загружаем файлы',
  repairing: 'Восстанавливаем файлы',
  extracting: 'Подготавливаем Java',
  ready: 'Minecraft и Java подготовлены',
  cancelling: 'Отменяем операцию',
  cancelled: 'Установка отменена',
  error: 'Установка не завершена',
};

export type Progress = {
  stage: keyof typeof stageLabels;
  completedBytes: number;
  totalBytes: number | null;
  downloadedBytes: number;
  elapsedMs: number;
};

export type Status = Progress & {
  operationId: string;
  rootId: string;
  profileId: string;
  profileName: string;
  sequence: number;
  errorCode: string | null;
  terminal: boolean;
};

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_response');
  return value as Record<string, unknown>;
}

function counter(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

export function parseProgress(value: unknown): Progress {
  const v = record(value);
  if (
    typeof v.stage !== 'string' ||
    !Object.hasOwn(stageLabels, v.stage) ||
    !counter(v.completedBytes) ||
    (v.totalBytes !== null && (!counter(v.totalBytes) || v.completedBytes > v.totalBytes)) ||
    !counter(v.downloadedBytes) ||
    !counter(v.elapsedMs)
  )
    throw new Error('invalid_progress');
  return {
    stage: v.stage as Progress['stage'],
    completedBytes: v.completedBytes,
    totalBytes: v.totalBytes,
    downloadedBytes: v.downloadedBytes,
    elapsedMs: v.elapsedMs,
  };
}

export function parseInstallation(value: unknown): Status | null {
  if (value === null) return null;
  const v = record(value);
  if (
    v.schemaVersion !== 1 ||
    typeof v.operationId !== 'string' ||
    typeof v.rootId !== 'string' ||
    typeof v.profileId !== 'string' ||
    typeof v.profileName !== 'string' ||
    !counter(v.sequence) ||
    typeof v.terminal !== 'boolean' ||
    (v.errorCode !== null && typeof v.errorCode !== 'string')
  )
    throw new Error('invalid_response');
  return {
    ...parseProgress(v),
    operationId: v.operationId,
    rootId: v.rootId,
    profileId: v.profileId,
    profileName: v.profileName,
    sequence: v.sequence,
    errorCode: v.errorCode,
    terminal: v.terminal,
  };
}
