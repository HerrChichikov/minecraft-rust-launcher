import type { ClientErrorCode, FoundationSnapshot } from '../../shared/ipc/types';
import type { FoundationState } from './useFoundation';

const osLabels = { windows: 'Windows', linux: 'Linux', macos: 'macOS' } as const;
const archLabels = { x86_64: 'x64', aarch64: 'ARM64' } as const;

const errorMessages: Record<ClientErrorCode, { title: string; description: string }> = {
  desktop_required: {
    title: 'Откройте desktop-приложение',
    description:
      'Эта страница открыта в браузере. Для подключения к Rust-ядру запустите npm run dev в проекте.',
  },
  backend_unavailable: {
    title: 'Не удалось подключиться к ядру',
    description: 'Повторите попытку. Если ошибка сохраняется, перезапустите приложение.',
  },
  unsupported_platform: {
    title: 'Платформа не поддерживается',
    description: 'Используйте сборку для Windows x64, Linux x64 или macOS Intel / ARM64.',
  },
  invalid_response: {
    title: 'Версии интерфейса и ядра не совпадают',
    description: 'Пересоберите desktop-приложение и запустите его снова.',
  },
};

function BuildDetails({ snapshot }: { snapshot: FoundationSnapshot }) {
  return (
    <dl className="build-details">
      <div>
        <dt>Версия лаунчера</dt>
        <dd>{snapshot.launcherVersion}</dd>
      </div>
      <div>
        <dt>Платформа сборки</dt>
        <dd>
          {osLabels[snapshot.platform.os]} · {archLabels[snapshot.platform.architecture]}
        </dd>
      </div>
      <div>
        <dt>Режим</dt>
        <dd>{snapshot.developmentBuild ? 'Development' : 'Release'}</dd>
      </div>
    </dl>
  );
}

export function FoundationStatus({ state, retry }: { state: FoundationState; retry: () => void }) {
  if (state.status === 'loading') {
    return (
      <section className="status-card" aria-label="Состояние приложения" aria-busy="true">
        <p className="eyebrow">Состояние приложения</p>
        <div role="status" className="connection-line">
          <span className="status-dot status-dot--pending" />
          Подключение к ядру…
        </div>
      </section>
    );
  }

  if (state.status === 'error') {
    const message = errorMessages[state.code];
    return (
      <section className="status-card status-card--error" aria-label="Состояние приложения">
        <p className="eyebrow">Состояние приложения</p>
        <div role="alert">
          <h2>{message.title}</h2>
          <p className="muted">{message.description}</p>
        </div>
        {state.code !== 'desktop_required' && (
          <button className="button button--secondary" type="button" onClick={retry}>
            Повторить подключение
          </button>
        )}
      </section>
    );
  }

  return (
    <section className="status-card" aria-label="Состояние приложения">
      <p className="eyebrow">Состояние приложения</p>
      <div className="connection-line" role="status">
        <span className="status-dot" />
        Ядро подключено
      </div>
      <BuildDetails snapshot={state.snapshot} />
    </section>
  );
}
