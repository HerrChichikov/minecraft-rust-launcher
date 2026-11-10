import { useEffect, useRef, useState } from 'react';
import { AuthPanel } from '../features/auth/AuthPanel';
import { useAuth } from '../features/auth/useAuth';
import { FoundationStatus } from '../features/foundation/FoundationStatus';
import { useFoundation } from '../features/foundation/useFoundation';
import { GamePanel } from '../features/game/GamePanel';
import { useGame } from '../features/game/useGame';
import { InstallationPanel } from '../features/profiles/InstallationPanel';
import { ProfilesPanel } from '../features/profiles/ProfilesPanel';
import { useInstallation } from '../features/profiles/useInstallation';
import { useWorkspace } from '../features/profiles/useWorkspace';
import type { FoundationSnapshot } from '../shared/ipc/types';
import { WorldArt } from './WorldArt';

const screens = {
  play: { title: 'Игра', icon: 'm8 5 12 7-12 7z' },
  account: {
    title: 'Аккаунт',
    icon: 'M16 8a4 4 0 1 1-8 0 4 4 0 0 1 8 0ZM4 21v-2a8 6 0 0 1 16 0v2',
  },
  settings: { title: 'Настройки', icon: 'M4 7h16M4 17h16M9 4v6m6 4v6' },
};

type Screen = keyof typeof screens;

function Brand() {
  return (
    <div className="brand">
      <span className="brand-mark" aria-hidden="true">
        <svg viewBox="0 0 28 28" fill="none">
          <path
            d="m14 3 10 5.5v11L14 25 4 19.5v-11L14 3Zm0 11 10-5.5M14 14 4 8.5M14 14v11"
            stroke="currentColor"
            strokeWidth="1.7"
            strokeLinejoin="round"
          />
        </svg>
      </span>
      <span>
        Launcher<small>Minecraft</small>
      </span>
    </div>
  );
}

function Launcher({ snapshot }: { snapshot: FoundationSnapshot }) {
  const [screen, setScreen] = useState<Screen>('play');
  const heading = useRef<HTMLHeadingElement>(null);
  const moveFocus = useRef(false);
  const auth = useAuth();
  const workspaceModel = useWorkspace();
  const installation = useInstallation();
  const game = useGame();
  const { workspace } = workspaceModel;
  const profile = workspace?.profiles.find((item) => item.id === workspace.activeProfileId);
  const account = auth.snapshot?.account;
  const operationActive = installation.active || game.active;
  const prepared = Boolean(
    workspace?.installationRecorded ||
    (workspace &&
      installation.status?.stage === 'ready' &&
      installation.status.terminal &&
      installation.status.rootId === workspace.rootId &&
      installation.status.profileId === workspace.activeProfileId) ||
    (workspace &&
      game.status?.preparation?.stage === 'ready' &&
      game.status.rootId === workspace.rootId &&
      game.status.profileId === workspace.activeProfileId),
  );

  function navigate(next: Screen) {
    if (next === screen) return;
    moveFocus.current = true;
    setScreen(next);
  }

  useEffect(() => {
    if (moveFocus.current) {
      heading.current?.focus();
      moveFocus.current = false;
    }
  }, [screen]);

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Перейти к содержимому
      </a>
      <aside className="sidebar">
        <Brand />
        <nav aria-label="Разделы лаунчера" className="navigation">
          {(Object.keys(screens) as Screen[]).map((item) => (
            <button
              className="nav-link"
              key={item}
              type="button"
              aria-current={screen === item ? 'page' : undefined}
              onClick={() => navigate(item)}
            >
              <svg
                aria-hidden="true"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.7"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d={screens[item].icon} />
              </svg>
              {screens[item].title}
              {item === 'play' && operationActive && (
                <span className="nav-dot" aria-label="Выполняется операция" />
              )}
            </button>
          ))}
        </nav>
        <button
          className="account-chip"
          type="button"
          onClick={() => navigate('account')}
          aria-label={account ? `Аккаунт ${account.playerName}` : 'Войти в аккаунт'}
        >
          <span className="avatar" aria-hidden="true">
            {account ? account.playerName.slice(0, 2).toUpperCase() : '◈'}
          </span>
          <span className="account-text">
            <strong>{account?.playerName ?? 'Гость'}</strong>
            <small>
              {account?.development ? 'Тестовый вход' : account ? 'Ely.by' : 'Войти в аккаунт'}
            </small>
          </span>
        </button>
      </aside>

      <main className="main-content" id="main-content" tabIndex={-1}>
        <header className="page-header">
          <h1 ref={heading} tabIndex={-1}>
            {screens[screen].title}
          </h1>
          <span className="badge">
            {snapshot.developmentBuild ? 'Development' : 'Java Edition'}
          </span>
        </header>
        {screen === 'play' && (
          <>
            <section className="hero" aria-labelledby="game-title">
              <WorldArt />
              <div className="hero-shade" />
              <div className="hero-content">
                <p className="eyebrow hero-edition">
                  JAVA EDITION <span aria-hidden="true">·</span>{' '}
                  {profile?.minecraftVersion ?? snapshot.targetMinecraftVersion}
                </p>
                <h2 id="game-title">Minecraft</h2>
                <p className="hero-profile">
                  {profile?.name ??
                    (workspaceModel.busy ? 'Загружаем профиль…' : 'Профиль недоступен')}
                </p>
                {workspaceModel.error && (
                  <div>
                    <p role="alert" className="storage-error">
                      {workspaceModel.error}
                    </p>
                    <button
                      className="button button--secondary"
                      type="button"
                      onClick={() => navigate('settings')}
                    >
                      Проверить настройки
                    </button>
                  </div>
                )}
                <GamePanel
                  model={game}
                  workspace={workspace}
                  available={snapshot.features.gameLaunch}
                  signedIn={Boolean(account)}
                  onSignIn={() => navigate('account')}
                  installActive={installation.active}
                  controlsBusy={workspaceModel.busy || auth.busy}
                />
              </div>
            </section>
            <div className="overview-grid">
              <InstallationPanel
                model={installation}
                workspace={workspace}
                gameActive={game.active || workspaceModel.busy}
                prepared={prepared}
              />
              <section className="panel profile-summary" aria-labelledby="profile-summary-title">
                <p className="eyebrow">Активный профиль</p>
                <h2 id="profile-summary-title">{profile?.name ?? 'Настройки профиля'}</h2>
                {profile && (
                  <p className="muted">
                    {profile.settings.runtime.kind === 'managed' ? 'Управляемая Java' : 'Своя Java'}{' '}
                    · RAM до {profile.settings.maxRamMib.toLocaleString('ru-RU')} МиБ
                  </p>
                )}
                <button
                  className="button button--secondary"
                  type="button"
                  onClick={() => navigate('settings')}
                >
                  Настроить профиль
                </button>
              </section>
            </div>
          </>
        )}
        {screen === 'account' && <AuthPanel model={auth} />}
        <div hidden={screen !== 'settings'}>
          <ProfilesPanel model={workspaceModel} operationActive={operationActive} />
          <details className="build-info">
            <summary>О лаунчере</summary>
            <FoundationStatus state={{ status: 'ready', snapshot }} retry={() => {}} />
          </details>
        </div>
      </main>
    </div>
  );
}

export function App() {
  const { state, retry } = useFoundation();
  if (state.status === 'ready') return <Launcher snapshot={state.snapshot} />;
  return (
    <main className="boot-screen">
      <Brand />
      <FoundationStatus state={state} retry={retry} />
    </main>
  );
}
