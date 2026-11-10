import { useState } from 'react';
import type { useAuth } from './useAuth';
import {
  getAuth,
  login,
  loginPassword,
  continueLogin,
  cancelLogin,
  refreshAuth,
  restoreAuth,
  logout,
} from './client';
export function AuthPanel({ model }: { model: ReturnType<typeof useAuth> }) {
  const { snapshot, busy, error, run } = model;
  const [provider, setProvider] = useState<'ely' | 'mock'>();
  const [nickname, setNickname] = useState('');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [remember, setRemember] = useState(false);
  const [totp, setTotp] = useState('');
  const mock = snapshot?.providers.some((p) => p.id === 'development-mock' && p.available);
  const ely = snapshot?.providers.some((p) => p.id === 'ely-by' && p.available);
  const useMock = mock && (provider === 'mock' || !ely);
  return (
    <section className="panel auth-card" aria-labelledby="auth-title" aria-busy={busy}>
      <div className="section-heading">
        <div>
          <h2 id="auth-title">Вход в игру</h2>
        </div>
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
            <p className="development-notice">
              Development / Mock доступен только для тестирования, без входа в Ely.by.
            </p>
          )}
          {snapshot.notice === 'credential_store_unavailable' && (
            <p role="status" className="storage-error">
              Вход выполнен, но сохранить токены в системном хранилище не удалось. Эта сессия
              останется только в памяти.
            </p>
          )}
          {(snapshot.notice === 'credential_delete_failed' ||
            snapshot.notice === 'logout_cleanup_failed') && (
            <p role="status" className="storage-error">
              Локальный вход завершён, но очистить системное хранилище не удалось.{' '}
              {snapshot.notice === 'logout_cleanup_failed' &&
                'Отзыв сессии в Ely.by также не подтверждён.'}{' '}
              Повторите удаление сохранённой сессии.
            </p>
          )}
          {snapshot.notice === 'remote_logout_failed' && (
            <p role="status" className="storage-error">
              Вы вышли из приложения. Ely.by не подтвердил отзыв удалённой сессии.
            </p>
          )}
          {snapshot.challenge ? (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                const code = totp;
                setTotp('');
                void run(() => continueLogin(snapshot, code));
              }}
            >
              <p>
                Для входа в Ely.by нужен код двухфакторной аутентификации. Если время истекло,
                начните вход заново.
              </p>
              <label>
                Код TOTP
                <input
                  autoComplete="one-time-code"
                  inputMode="numeric"
                  type="password"
                  value={totp}
                  onChange={(e) => setTotp(e.target.value)}
                  pattern="[0-9]{6}"
                  maxLength={6}
                  required
                  disabled={busy}
                />
              </label>
              <div className="installation-actions">
                <button
                  className="button button--primary"
                  type="submit"
                  disabled={busy || !/^\d{6}$/.test(totp)}
                >
                  Подтвердить код
                </button>
                <button
                  className="button"
                  type="button"
                  disabled={busy}
                  onClick={() => {
                    setTotp('');
                    void run(() => cancelLogin(snapshot));
                  }}
                >
                  Отменить вход
                </button>
              </div>
            </form>
          ) : snapshot.account ? (
            <div className="auth-account">
              <p role="status">
                {snapshot.account.development
                  ? 'Выполнен тестовый вход:'
                  : 'Вход в Ely.by выполнен:'}{' '}
                <strong>{snapshot.account.playerName}</strong>
              </p>

              <div className="installation-actions">
                <button
                  className="button"
                  type="button"
                  disabled={busy}
                  onClick={() => void run(() => refreshAuth(snapshot))}
                >
                  Обновить сессию
                </button>
                <button
                  className="button"
                  type="button"
                  disabled={busy}
                  onClick={() => void run(() => logout(snapshot))}
                >
                  Выйти
                </button>
              </div>
            </div>
          ) : (
            <>
              {mock && ely && (
                <div className="provider-switch" role="group" aria-label="Способ входа">
                  <button
                    className="button"
                    type="button"
                    aria-pressed={!useMock}
                    disabled={busy}
                    onClick={() => setProvider('ely')}
                  >
                    Ely.by
                  </button>
                  <button
                    className="button"
                    type="button"
                    aria-pressed={useMock}
                    disabled={busy}
                    onClick={() => {
                      setPassword('');
                      setProvider('mock');
                    }}
                  >
                    Тестовый вход
                  </button>
                </div>
              )}
              {ely && !useMock && (
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    const secret = password;
                    setPassword('');
                    void run(() => loginPassword(snapshot, username, secret, remember));
                  }}
                >
                  <h3>Ely.by</h3>
                  <p className="field-help">
                    Логин и пароль отправляются напрямую Ely.by. Пароль и код не сохраняются.
                  </p>
                  <label>
                    Логин или email Ely.by
                    <input
                      autoComplete="username"
                      value={username}
                      onChange={(e) => setUsername(e.target.value)}
                      maxLength={254}
                      required
                      disabled={busy}
                    />
                  </label>
                  <label>
                    Пароль Ely.by
                    <input
                      autoComplete="current-password"
                      type="password"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      maxLength={4096}
                      required
                      disabled={busy}
                    />
                  </label>
                  {snapshot.rememberSupported && (
                    <label className="remember-choice">
                      <input
                        type="checkbox"
                        checked={remember}
                        onChange={(e) => setRemember(e.target.checked)}
                        disabled={busy}
                      />
                      Сохранить токены в системном защищённом хранилище
                    </label>
                  )}
                  <button
                    className="button button--primary"
                    type="submit"
                    disabled={busy || !username || !password}
                  >
                    {busy ? 'Выполняется вход…' : 'Войти через Ely.by'}
                  </button>
                </form>
              )}
              {!ely && !mock && <p className="muted">Вход временно недоступен.</p>}
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
            </>
          )}
          {busy && <p role="status">Связываемся с провайдером…</p>}
          {snapshot.rememberSupported && !snapshot.account && !snapshot.challenge && !useMock && (
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
          <p className="field-help auth-help">
            {snapshot.persistence === 'system_credential_store'
              ? 'Токены сохранены в системном защищённом хранилище. После перезапуска их можно восстановить отдельной кнопкой.'
              : 'Текущая сессия хранится в памяти до закрытия приложения.'}{' '}
            Пароль и TOTP не сохраняются. После входа нажмите «Играть».
          </p>
        </>
      )}
    </section>
  );
}
