import { useState } from 'react';
import type { useAuth } from './useAuth';
import {
  getAuth,
  login,
  loginPassword,
  logout,
  refreshAuth,
  registerAccount,
  restoreAuth,
  selectGameProfile,
} from './client';

const notices = {
  registered: 'Аккаунт создан. Теперь можно войти.',
  game_profiles_unavailable: 'Не удалось загрузить игровые профили. Обновите список.',
  credential_store_unavailable:
    'Не удалось сохранить сессию в системном хранилище. Она доступна до закрытия приложения.',
  credential_delete_failed: 'Не удалось удалить сохранённую сессию. Повторите удаление.',
  logout_cleanup_failed:
    'Сервер не подтвердил отзыв сессии, а системное хранилище не удалило её. Повторите удаление.',
  remote_logout_failed: 'Вы вышли из приложения. Сервер не подтвердил отзыв удалённой сессии.',
};

export function AuthPanel({ model }: { model: ReturnType<typeof useAuth> }) {
  const { snapshot, busy, error, run } = model;
  const [mode, setMode] = useState<'login' | 'register' | 'mock'>('login');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [email, setEmail] = useState('');
  const [invitationCode, setInvitationCode] = useState('');
  const [remember, setRemember] = useState(false);
  const [nickname, setNickname] = useState('');
  const mock = snapshot?.providers.some((p) => p.id === 'development-mock' && p.available);
  const platform = snapshot?.providers.some((p) => p.id === 'platform' && p.available);
  const useMock = mock && (mode === 'mock' || !platform);
  const registering = mode === 'register';

  function switchMode(next: typeof mode) {
    setPassword('');
    setInvitationCode('');
    setEmail('');
    setMode(next);
  }

  return (
    <section className="panel auth-card" aria-labelledby="auth-title" aria-busy={busy}>
      <div className="section-heading">
        <h2 id="auth-title">Вход в игру</h2>
        <button className="button" type="button" disabled={busy} onClick={() => void run(getAuth)}>
          Обновить состояние входа
        </button>
      </div>
      {error && (
        <p className="storage-error" role="alert">
          {error}
        </p>
      )}
      {!snapshot && !error && <p role="status">Проверяем доступные способы входа…</p>}
      {snapshot && (
        <>
          {mock && (
            <p className="development-notice">Development / Mock доступен для тестирования.</p>
          )}
          {snapshot.notice && <p role="status">{notices[snapshot.notice]}</p>}
          {snapshot.account ? (
            <>
              <p role="status">
                {snapshot.account.development ? 'Выполнен тестовый вход' : 'Вы вошли в аккаунт'}{' '}
                <strong>{snapshot.account.playerName}</strong>
              </p>
              {!snapshot.account.development && (
                <>
                  {snapshot.gameProfiles.length ? (
                    <label>
                      Игровой профиль
                      <select
                        value={snapshot.selectedGameProfileId ?? ''}
                        disabled={busy}
                        onChange={(e) => {
                          const id = e.currentTarget.value;
                          void run(() => selectGameProfile(snapshot, id));
                        }}
                      >
                        <option value="" disabled>
                          Выберите профиль
                        </option>
                        {snapshot.gameProfiles.map((p) => (
                          <option key={p.id} value={p.id}>
                            {p.nickname} · {p.realm}
                          </option>
                        ))}
                      </select>
                    </label>
                  ) : (
                    <p className="muted">У аккаунта пока нет доступных игровых профилей.</p>
                  )}
                  <button
                    className="button"
                    type="button"
                    disabled={busy}
                    onClick={() => void run(() => refreshAuth(snapshot))}
                  >
                    Обновить профили и сессию
                  </button>
                </>
              )}
              <button
                className="button"
                type="button"
                disabled={busy}
                onClick={() => void run(() => logout(snapshot))}
              >
                Выйти
              </button>
            </>
          ) : (
            <>
              {platform && (
                <div className="provider-switch">
                  <button
                    className="button"
                    type="button"
                    disabled={busy}
                    aria-pressed={!useMock && !registering}
                    onClick={() => switchMode('login')}
                  >
                    Вход
                  </button>
                  <button
                    className="button"
                    type="button"
                    disabled={busy}
                    aria-pressed={registering}
                    onClick={() => switchMode('register')}
                  >
                    Регистрация
                  </button>
                  {mock && (
                    <button
                      className="button"
                      type="button"
                      disabled={busy}
                      aria-pressed={useMock}
                      onClick={() => switchMode('mock')}
                    >
                      Тестовый вход
                    </button>
                  )}
                </div>
              )}
              {platform && !useMock && (
                <form
                  key={registering ? 'registration' : 'login'}
                  onSubmit={(e) => {
                    e.preventDefault();
                    const secret = password;
                    setPassword('');
                    if (registering) {
                      const fields = {
                        nickname: username,
                        password: secret,
                        email,
                        invitationCode,
                      };
                      setInvitationCode('');
                      setEmail('');
                      void run(() => registerAccount(snapshot, fields)).then((created) => {
                        if (created) setMode('login');
                      });
                    } else {
                      void run(() => loginPassword(snapshot, username, secret, remember));
                    }
                  }}
                >
                  <label>
                    {registering ? 'Логин' : 'Логин или email'}
                    <input
                      name="username"
                      autoComplete="username"
                      required
                      minLength={registering ? 3 : undefined}
                      maxLength={registering ? 50 : 254}
                      disabled={busy}
                      value={username}
                      onChange={(e) => setUsername(e.target.value)}
                    />
                  </label>
                  <label>
                    Пароль
                    <input
                      name="password"
                      type="password"
                      autoComplete={registering ? 'new-password' : 'current-password'}
                      required
                      minLength={registering ? 8 : undefined}
                      maxLength={72}
                      disabled={busy}
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                    />
                  </label>
                  {registering ? (
                    <>
                      <label>
                        Email (необязательно)
                        <input
                          name="email"
                          type="email"
                          autoComplete="email"
                          maxLength={254}
                          value={email}
                          disabled={busy}
                          onChange={(e) => setEmail(e.target.value)}
                        />
                      </label>
                      <label>
                        Код приглашения
                        <input
                          name="invitationCode"
                          autoComplete="off"
                          required
                          maxLength={20}
                          disabled={busy}
                          value={invitationCode}
                          onChange={(e) => setInvitationCode(e.target.value)}
                        />
                      </label>
                    </>
                  ) : (
                    snapshot.rememberSupported && (
                      <label className="remember-choice">
                        <input
                          type="checkbox"
                          checked={remember}
                          disabled={busy}
                          onChange={(e) => setRemember(e.target.checked)}
                        />
                        Запомнить на этом устройстве
                      </label>
                    )
                  )}
                  <button className="button button--primary" type="submit" disabled={busy}>
                    {registering ? 'Создать аккаунт' : 'Войти'}
                  </button>
                </form>
              )}
              {!platform && !mock && (
                <p className="muted">Сервер авторизации не настроен в этой сборке.</p>
              )}
              {useMock && (
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    void run(() => login(snapshot, nickname));
                  }}
                >
                  <label>
                    Тестовый ник
                    <input
                      autoComplete="off"
                      name="developmentNickname"
                      value={nickname}
                      onChange={(e) => setNickname(e.target.value)}
                      minLength={3}
                      maxLength={16}
                      pattern="[A-Za-z0-9_]{3,16}"
                      required
                      disabled={busy}
                    />
                  </label>
                  <button
                    className="button"
                    type="submit"
                    disabled={busy || !/^[A-Za-z0-9_]{3,16}$/.test(nickname)}
                  >
                    Войти в Development / Mock
                  </button>
                </form>
              )}
              {snapshot.rememberSupported && !registering && !useMock && (
                <div className="installation-actions">
                  <button
                    className="button"
                    type="button"
                    disabled={busy}
                    onClick={() => void run(() => restoreAuth(snapshot))}
                  >
                    Восстановить сохранённую сессию
                  </button>
                  <button
                    className="button"
                    type="button"
                    disabled={busy}
                    onClick={() => void run(() => logout(snapshot))}
                  >
                    Удалить сохранённую сессию
                  </button>
                </div>
              )}
            </>
          )}
          {busy && <p role="status">Обновляем данные аккаунта…</p>}
          {snapshot.persistence === 'system_credential_store' && (
            <p className="field-help auth-help">
              Сессия сохранена в системном хранилище. Её можно восстановить после перезапуска.
            </p>
          )}
        </>
      )}
    </section>
  );
}
