import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';
import { AuthError, getAuth, parseAuth, type AuthSnapshot } from './client';

const messages: Record<string, string> = {
  auth_unavailable: 'Выбранный способ входа пока недоступен.',
  revision_conflict: 'Состояние сессии изменилось. Обновите его перед повтором.',
  invalid_settings:
    'Проверьте введённые данные. Тестовый ник: 3–16 латинских букв, цифр или знаков подчёркивания.',
  auth_rejected: 'Проверьте логин и пароль или данные регистрации.',
  auth_network: 'Не удалось связаться с сервером авторизации. Попробуйте ещё раз.',
  auth_protocol: 'Сервер авторизации вернул неподдерживаемый ответ. Попробуйте позже.',
  auth_throttled: 'Слишком много попыток. Попробуйте позже.',
  game_profile_unavailable: 'Обновите список и выберите доступный игровой профиль.',
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
  const accept = useCallback((s: AuthSnapshot) => {
    if (s.revision > revision.current) {
      revision.current = s.revision;
      setSnapshot(s);
    }
  }, []);
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        if (isTauri()) {
          unlisten = await listen<unknown>('launcher://auth', ({ payload }) => {
            if (!active) return;
            try {
              accept(parseAuth(payload));
            } catch {
              setError('Не удалось прочитать обновление сессии.');
            }
          });
          if (!active) {
            unlisten();
            return;
          }
        }
        const initial = await getAuth();
        if (active) accept(initial);
      } catch {
        if (active && revision.current < 0)
          setError('Не удалось получить состояние входа. Попробуйте обновить.');
      }
    })();
    return () => {
      active = false;
      unlisten?.();
    };
  }, [accept]);
  async function run(action: () => Promise<AuthSnapshot>) {
    if (pending.current) return false;
    pending.current = true;
    setBusy(true);
    setError(undefined);
    try {
      accept(await action());
      return true;
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
      return false;
    } finally {
      pending.current = false;
      setBusy(false);
    }
  }
  return { snapshot, busy, error, run };
}
