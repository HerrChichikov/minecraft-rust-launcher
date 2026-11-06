import { useEffect, useRef, useState } from 'react';
import {
  createProfile,
  getWorkspace,
  saveProfile,
  selectProfile,
  StorageError,
  switchDataRoot,
} from './client';
import type { Profile, Workspace } from './client';

function errorMessage(error: unknown) {
  return error instanceof StorageError ? error.message : new StorageError().message;
}

function ProfileEditor({
  workspace,
  profile,
  busy,
  run,
}: {
  workspace: Workspace;
  profile: Profile;
  busy: boolean;
  run: (action: () => Promise<Workspace>, notice?: string) => Promise<void>;
}) {
  const [name, setName] = useState(profile.name);
  const [min, setMin] = useState(String(profile.settings.minRamMib));
  const [max, setMax] = useState(String(profile.settings.maxRamMib));
  const [custom, setCustom] = useState(profile.settings.runtime.kind === 'custom');
  const [java, setJava] = useState(
    profile.settings.runtime.kind === 'custom' ? profile.settings.runtime.executable : '',
  );
  const [args, setArgs] = useState(profile.settings.extraJvmArgs.join('\n'));
  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        void run(async () => {
          const result = await saveProfile(workspace, profile, name, {
            minRamMib: Number(min),
            maxRamMib: Number(max),
            runtime: custom ? { kind: 'custom', executable: java } : { kind: 'managed' },
            extraJvmArgs: args
              .split('\n')
              .map((s) => s.trim())
              .filter(Boolean),
          });
          return result;
        }, 'Настройки сохранены');
      }}
    >
      <fieldset disabled={busy} className="profile-fields">
        <legend>Настройки профиля</legend>
        <label>
          Название
          <input value={name} onChange={(e) => setName(e.target.value)} required maxLength={64} />
        </label>
        <div className="ram-fields">
          <label>
            Минимум RAM, МиБ
            <input
              type="number"
              min="256"
              max={workspace.policy.maxRamMib}
              step="1"
              value={min}
              onChange={(e) => setMin(e.target.value)}
              required
            />
          </label>
          <label>
            Максимум RAM, МиБ
            <input
              type="number"
              min={min || '256'}
              max={workspace.policy.maxRamMib}
              step="1"
              value={max}
              onChange={(e) => setMax(e.target.value)}
              required
            />
          </label>
        </div>
        <p className="field-help">
          В системе доступно {workspace.policy.maxRamMib.toLocaleString('ru-RU')} МиБ. Оставьте
          часть памяти для ОС.
        </p>
        <label>
          Java
          <select
            value={custom ? 'custom' : 'managed'}
            onChange={(e) => setCustom(e.target.value === 'custom')}
          >
            <option value="managed">Управляемая Java</option>
            <option value="custom">Свой исполняемый файл</option>
          </select>
        </label>
        {custom && (
          <label>
            Абсолютный путь к Java
            <input
              value={java}
              onChange={(e) => setJava(e.target.value)}
              required
              placeholder="Полный путь к java / java.exe"
            />
          </label>
        )}
        <p className="field-help">Установка и проверка версии Java появятся на следующих этапах.</p>
        <label>
          JVM-параметры, по одному на строку
          <textarea
            value={args}
            onChange={(e) => setArgs(e.target.value)}
            rows={3}
            aria-describedby="jvm-help"
          />
        </label>
        <p className="field-help" id="jvm-help">
          Допустимы -XX:+UseG1GC, -XX:+UseZGC, -XX:+UseParallelGC, -XX:+DisableExplicitGC и
          -XX:MaxGCPauseMillis=1…10000. RAM задаётся выше.
        </p>
        <button className="button button--primary" type="submit">
          Сохранить настройки
        </button>
      </fieldset>
    </form>
  );
}

export function ProfilesPanel() {
  const [workspace, setWorkspace] = useState<Workspace>();
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [busy, setBusy] = useState(true);
  const [newName, setNewName] = useState('');
  const [rootPath, setRootPath] = useState('');
  const mounted = useRef(false);
  const pending = useRef(false);
  useEffect(() => {
    let active = true;
    mounted.current = true;
    void getWorkspace().then(
      (w) => {
        if (active) {
          setWorkspace(w);
          setRootPath(w.dataRoot);
          setBusy(false);
        }
      },
      (e: unknown) => {
        if (active) {
          setError(errorMessage(e));
          setBusy(false);
        }
      },
    );
    return () => {
      active = false;
      mounted.current = false;
    };
  }, []);
  async function run(action: () => Promise<Workspace>, success?: string) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const next = await action();
      if (mounted.current) {
        setNotice(success);
        setWorkspace(next);
        setRootPath(next.dataRoot);
      }
    } catch (e: unknown) {
      if (mounted.current) setError(errorMessage(e));
    } finally {
      pending.current = false;
      if (mounted.current) setBusy(false);
    }
  }
  const profile = workspace?.profiles.find((p) => p.id === workspace.activeProfileId);
  return (
    <section className="profiles-card" aria-labelledby="profiles-title" aria-busy={busy}>
      <div className="section-heading">
        <div>
          <p className="eyebrow">Ваши конфигурации</p>
          <h2 id="profiles-title">Профили и настройки</h2>
        </div>
        <button
          className="button button--secondary"
          disabled={busy}
          type="button"
          onClick={() => {
            void run(getWorkspace);
          }}
        >
          Обновить список
        </button>
      </div>
      {error && (
        <p role="alert" className="storage-error">
          {error}
        </p>
      )}
      {notice && <p role="status">{notice}</p>}
      {!workspace && busy && <p role="status">Загрузка профилей…</p>}
      {workspace && profile && (
        <>
          <div className="profile-layout">
            <div className="profile-list">
              <label>
                Активный профиль
                <select
                  disabled={busy}
                  value={workspace.activeProfileId}
                  onChange={(e) => {
                    void run(() => selectProfile(workspace, e.target.value));
                  }}
                >
                  {workspace.profiles.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </label>
              <p className="field-help">
                Minecraft {profile.minecraftVersion}. У каждого профиля своя игровая папка и
                сохранения.
              </p>
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  void run(async () => {
                    const result = await createProfile(workspace.rootId, newName);
                    setNewName('');
                    return result;
                  });
                }}
              >
                <label>
                  Название нового профиля
                  <input
                    disabled={busy}
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    required
                    maxLength={64}
                  />
                </label>
                <button
                  className="button button--secondary"
                  disabled={busy || !newName.trim()}
                  type="submit"
                >
                  Создать профиль
                </button>
              </form>
            </div>
            <ProfileEditor
              key={`${workspace.rootId}:${profile.id}:${profile.revision}`}
              workspace={workspace}
              profile={profile}
              busy={busy}
              run={run}
            />
          </div>
          <form
            className="root-settings"
            onSubmit={(e) => {
              e.preventDefault();
              void run(() => switchDataRoot(workspace, rootPath));
            }}
          >
            <h3>Папка данных</h3>
            <p className="data-path">{workspace.dataRoot}</p>
            <label>
              Новый путь
              <input
                disabled={busy}
                value={rootPath}
                onChange={(e) => setRootPath(e.target.value)}
                required
              />
            </label>
            <p className="field-help">
              Выберите пустую папку или существующее хранилище Launcher. Переключение не переносит
              файлы: прежние миры и настройки останутся в старой папке.
            </p>
            <button
              className="button button--secondary"
              type="submit"
              disabled={busy || rootPath === workspace.dataRoot}
            >
              Сменить папку
            </button>
          </form>
        </>
      )}
    </section>
  );
}
