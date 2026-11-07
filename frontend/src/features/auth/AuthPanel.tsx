import { useEffect, useRef, useState } from 'react';
import { AuthError, getAuth, login, logout, type AuthSnapshot } from './client';
const messages: Record<string, string> = {
  auth_unavailable: 'Выбранный способ входа пока недоступен.',
  revision_conflict: 'Состояние сессии изменилось. Обновите его перед повтором.',
  invalid_settings: 'Ник должен содержать 3–16 латинских букв, цифр или знаков подчёркивания.',
  desktop_required: 'Для входа откройте desktop-приложение.',
};
export function AuthPanel() {
  const [snapshot, setSnapshot] = useState<AuthSnapshot>();
  const [nickname, setNickname] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const revision = useRef(-1);
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
      const s = await action();
      if (s.revision >= revision.current) {
        revision.current = s.revision;
        setSnapshot(s);
      }
    } catch (e: unknown) {
      setError(
        e instanceof AuthError && messages[e.code]
          ? messages[e.code]
          : 'Не удалось выполнить действие. Попробуйте обновить состояние.',
      );
    } finally {
      setBusy(false);
    }
  }
  const mock = snapshot?.providers.some((p) => p.id === 'development-mock' && p.available);
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
              Development / Mock — тестовая сессия, без входа в Ely.by.
            </p>
          )}
          {snapshot.account ? (
            <div className="auth-account">
              <p role="status">
                Выполнен тестовый вход: <strong>{snapshot.account.playerName}</strong>
              </p>
              <p className="field-help">UUID: {snapshot.account.playerId}</p>
              <button
                className="button"
                type="button"
                disabled={busy}
                onClick={() => void run(() => logout(snapshot))}
              >
                Выйти
              </button>
            </div>
          ) : (
            <>
              <p className="muted">Ely.by: реальный вход пока недоступен.</p>
              {mock ? (
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
                    className="button button--primary"
                    type="submit"
                    disabled={busy || !/^[A-Za-z0-9_]{3,16}$/.test(nickname)}
                  >
                    {busy ? 'Выполняется вход…' : 'Войти в Development / Mock'}
                  </button>
                </form>
              ) : (
                <p className="field-help">Для этой сборки нет доступного провайдера авторизации.</p>
              )}
            </>
          )}
          <p className="field-help auth-help">
            Сессия хранится только до закрытия приложения. Запуск Minecraft появится на следующем
            этапе.
          </p>
        </>
      )}
    </section>
  );
}
