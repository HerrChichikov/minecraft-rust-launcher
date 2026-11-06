import { invoke, isTauri } from '@tauri-apps/api/core';

export interface LaunchSettings {
  minRamMib: number;
  maxRamMib: number;
  runtime: { kind: 'managed' } | { kind: 'custom'; executable: string };
  extraJvmArgs: string[];
}
export interface Profile {
  schemaVersion: 1;
  revision: number;
  id: string;
  name: string;
  minecraftVersion: '1.21.1';
  settings: LaunchSettings;
}
export interface Workspace {
  schemaVersion: 1;
  rootId: string;
  dataRoot: string;
  configurationRevision: number;
  revision: number;
  activeProfileId: string;
  profiles: Profile[];
  policy: { maxRamMib: number };
}

const messages = {
  desktop_required: 'Профили доступны в desktop-приложении.',
  invalid_response: 'Версии интерфейса и хранилища не совпадают. Пересоберите приложение.',
  storage_busy:
    'Хранилище или профиль заняты. Закройте другой экземпляр лаунчера или дождитесь завершения операции.',
  corrupt_document:
    'Файл конфигурации повреждён. Исходные данные сохранены; проверьте конфигурацию или восстановите её из резервной копии.',
  unsupported_schema: 'Эти настройки созданы более новой версией лаунчера. Обновите приложение.',
  revision_conflict: 'Настройки уже изменились. Обновите список перед повторным сохранением.',
  unsafe_path:
    'Папка не подходит: выберите пустой каталог или существующее хранилище Launcher без символьных ссылок.',
  invalid_settings: 'Проверьте имя, объём RAM, путь к Java и допустимые JVM-параметры.',
  durability_failed:
    'Изменение записано, но диск не подтвердил сохранность. Обновите список и проверьте состояние диска.',
  storage_failed:
    'Не удалось прочитать или записать настройки. Проверьте права доступа и свободное место.',
} as const;
export class StorageError extends Error {
  constructor(readonly code: keyof typeof messages = 'storage_failed') {
    super(messages[code]);
    this.name = 'StorageError';
  }
}
function record(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}
function positive(v: unknown): v is number {
  return Number.isSafeInteger(v) && (v as number) > 0 && (v as number) <= 0xffffffff;
}
function uuid(v: unknown): v is string {
  return (
    typeof v === 'string' &&
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(v)
  );
}
function settings(v: unknown): v is LaunchSettings {
  return (
    record(v) &&
    positive(v.minRamMib) &&
    positive(v.maxRamMib) &&
    v.minRamMib >= 256 &&
    v.minRamMib <= v.maxRamMib &&
    v.maxRamMib <= 1048576 &&
    record(v.runtime) &&
    (v.runtime.kind === 'managed' ||
      (v.runtime.kind === 'custom' && typeof v.runtime.executable === 'string')) &&
    Array.isArray(v.extraJvmArgs) &&
    v.extraJvmArgs.length <= 32 &&
    v.extraJvmArgs.every((arg: unknown) => typeof arg === 'string' && arg.length <= 512)
  );
}
function profile(v: unknown): v is Profile {
  return (
    record(v) &&
    v.schemaVersion === 1 &&
    positive(v.revision) &&
    uuid(v.id) &&
    typeof v.name === 'string' &&
    v.name.length > 0 &&
    v.name.length <= 128 &&
    v.minecraftVersion === '1.21.1' &&
    settings(v.settings)
  );
}
export function parseWorkspace(v: unknown): Workspace {
  if (
    !record(v) ||
    v.schemaVersion !== 1 ||
    !uuid(v.rootId) ||
    typeof v.dataRoot !== 'string' ||
    !positive(v.configurationRevision) ||
    !positive(v.revision) ||
    !uuid(v.activeProfileId) ||
    !Array.isArray(v.profiles) ||
    v.profiles.length === 0 ||
    v.profiles.length > 128 ||
    !v.profiles.every(profile) ||
    !v.profiles.some((p) => p.id === v.activeProfileId) ||
    new Set(v.profiles.map((p) => p.id)).size !== v.profiles.length ||
    !record(v.policy) ||
    !positive(v.policy.maxRamMib) ||
    v.policy.maxRamMib < 256 ||
    v.policy.maxRamMib > 1048576
  ) {
    throw new StorageError('invalid_response');
  }
  // Only documented fields cross the IPC trust boundary.
  return {
    schemaVersion: 1,
    rootId: v.rootId,
    dataRoot: v.dataRoot,
    configurationRevision: v.configurationRevision,
    revision: v.revision,
    activeProfileId: v.activeProfileId,
    profiles: v.profiles.map((p) => ({
      schemaVersion: 1,
      revision: p.revision,
      id: p.id,
      name: p.name,
      minecraftVersion: p.minecraftVersion,
      settings: {
        minRamMib: p.settings.minRamMib,
        maxRamMib: p.settings.maxRamMib,
        runtime:
          p.settings.runtime.kind === 'managed'
            ? { kind: 'managed' }
            : { kind: 'custom', executable: p.settings.runtime.executable },
        extraJvmArgs: [...p.settings.extraJvmArgs],
      },
    })),
    policy: { maxRamMib: v.policy.maxRamMib },
  };
}
async function call(command: string, request?: unknown): Promise<Workspace> {
  if (!isTauri()) throw new StorageError('desktop_required');
  let response: unknown;
  try {
    response = await invoke<unknown>(command, request === undefined ? undefined : { request });
  } catch (error: unknown) {
    const code =
      record(error) && typeof error.code === 'string' && Object.hasOwn(messages, error.code)
        ? (error.code as keyof typeof messages)
        : 'storage_failed';
    throw new StorageError(code);
  }
  return parseWorkspace(response);
}
export const getWorkspace = () => call('get_workspace');
export const createProfile = (rootId: string, name: string) =>
  call('create_profile', { rootId, name });
export const selectProfile = (w: Workspace, profileId: string) =>
  call('select_profile', { rootId: w.rootId, profileId, expectedRevision: w.revision });
export const saveProfile = (w: Workspace, p: Profile, name: string, value: LaunchSettings) =>
  call('save_profile', {
    rootId: w.rootId,
    profileId: p.id,
    expectedRevision: p.revision,
    name,
    settings: value,
  });
export const switchDataRoot = (w: Workspace, path: string) =>
  call('switch_data_root', { rootId: w.rootId, expectedRevision: w.configurationRevision, path });
