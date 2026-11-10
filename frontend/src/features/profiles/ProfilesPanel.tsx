import { useState } from 'react';
import type { useWorkspace } from './useWorkspace';
import { createProfile, getWorkspace, saveProfile, selectProfile, switchDataRoot } from './client';
import type { Profile, Workspace } from './client';

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
        <p className="field-help">
          Управляемая Java устанавливается вместе с Minecraft. Выбранная Java проверяется при
          установке.
        </p>
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

export function ProfilesPanel({
  model,
  operationActive,
}: {
  model: ReturnType<typeof useWorkspace>;
  operationActive: boolean;
}) {
  const { workspace, busy: saving, error, notice, rootPath, setRootPath, run } = model;
  const busy = saving || operationActive;
  const [newName, setNewName] = useState('');
  const profile = workspace?.profiles.find((p) => p.id === workspace.activeProfileId);
  return (
    <section className="panel profiles-card" aria-labelledby="profiles-title" aria-busy={busy}>
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
      {operationActive && (
        <p className="field-help">Настройки можно изменить после завершения текущей операции.</p>
      )}
      {error && (
        <p role="alert" className="storage-error">
          {error}
        </p>
      )}
      {notice && <p role="status">{notice}</p>}
      {!workspace && saving && <p role="status">Загрузка профилей…</p>}
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
