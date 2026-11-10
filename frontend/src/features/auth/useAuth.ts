import { useEffect, useRef, useState } from 'react';
import { AuthError, getAuth, type AuthSnapshot } from './client';

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
  credential_store_unavailable:
    'Системное хранилище недоступно или заблокировано. Можно войти на текущую сессию.',
  saved_session_missing: 'Сохранённой сессии нет. Введите логин и пароль.',
  desktop_required: 'Для входа откройте desktop-приложение.',
};
export function useAuth() {
  const [snapshot, setSnapshot] = useState<AuthSnapshot>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const revision = useRef(-1);
  const pending = useRef(false);
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
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(undefined);
    try {
      accept(await action());
    } catch (e: unknown) {
      setError(
        e instanceof AuthError && Object.hasOwn(messages, e.code)
          ? messages[e.code]
          : 'Не удалось выполнить действие. Попробуйте обновить состояние.',
      );
      try {
        accept(await getAuth());
      } catch {
        /* Preserve known state until an explicit refresh succeeds. */
      }
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  return { snapshot, busy, error, run };
}
