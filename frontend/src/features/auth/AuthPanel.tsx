import { useEffect, useRef, useState } from 'react';
import {
  AuthError,
  getAuth,
  login,
  loginPassword,
  continueLogin,
  cancelLogin,
  refreshAuth,
  logout,
  type AuthSnapshot,
} from './client';
const messages: Record<string, string> = {
  auth_unavailable: 'Выбранный способ входа пока недоступен.',
  revision_conflict: 'Состояние сессии изменилось. Обновите его перед повтором.',
  invalid_settings:
    'Проверьте введённые данные. Тестовый ник: 3–16 латинских букв, цифр или знаков подчёркивания.',
  auth_rejected: 'Ely.by отклонил данные входа. Проверьте логин, пароль или код.',
  auth_network: 'Не удалось связаться с Ely.by. Попробуйте ещё раз.',
  auth_protocol: 'Ely.by вернул неподдерживаемый ответ. Попробуйте позже.',
  auth_challenge_expired: 'Время ввода кода истекло. Введите логин и пароль заново.',
  auth_busy: 'Вход уже выполняется. Дождитесь завершения и обновите состояние.',
  authentication_required: 'Сессия истекла. Выйдите и выполните вход заново.',
  desktop_required: 'Для входа откройте desktop-приложение.',
};
export function AuthPanel() {
  const [snapshot, setSnapshot] = useState<AuthSnapshot>();
  const [nickname, setNickname] = useState('');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [totp, setTotp] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const revision = useRef(-1);
  function accept(s: AuthSnapshot) {
    if (s.revision >= revision.current) {
      revision.current = s.revision;
      setSnapshot(s);
    }
  }
  useEffect(() => {
    let active = true;
    void getAuth()
      .then((s) => {
        if (active && s.revision >= revision.current) {
          revision.current = s.revision;
          setSnapshot(s);
        }
      })
      .catch(() => {
        if (active && revision.current < 0)
          setError('Не удалось получить состояние входа. Попробуйте обновить.');
      });
    return () => {
      active = false;
    };
  }, []);
  async function run(action: () => Promise<AuthSnapshot>) {
    setBusy(true);
    setError(undefined);
    try {
      accept(await action());
    } catch (e: unknown) {
      setError(
        e instanceof AuthError && messages[e.code]
          ? messages[e.code]
          : 'Не удалось выполнить действие. Попробуйте обновить состояние.',
      );
      try {
        accept(await getAuth());
      } catch {
        /* Preserve known state until an explicit refresh succeeds. */
      }
    } finally {
      setBusy(false);
    }
  }
  const mock = snapshot?.providers.some((p) => p.id === 'development-mock' && p.available);
  const ely = snapshot?.providers.some((p) => p.id === 'ely-by' && p.available);
  return (
    <section className="profiles-card auth-card" aria-labelledby="auth-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Аккаунт</p>
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
              <p className="field-help">UUID: {snapshot.account.playerId}</p>
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
              {ely ? (
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    const secret = password;
                    setPassword('');
                    void run(() => loginPassword(snapshot, username, secret));
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
                  <button
                    className="button button--primary"
                    type="submit"
                    disabled={busy || !username || !password}
                  >
                    {busy ? 'Выполняется вход…' : 'Войти через Ely.by'}
                  </button>
                </form>
              ) : (
                <p className="muted">Ely.by: реальный вход пока недоступен.</p>
              )}
              {mock && (
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
          <p className="field-help auth-help">
            Сессия хранится только до закрытия приложения. Запуск Minecraft появится на следующем
            этапе.
          </p>
        </>
      )}
    </section>
  );
}
