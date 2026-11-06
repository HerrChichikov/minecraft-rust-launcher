import { FoundationStatus } from '../features/foundation/FoundationStatus';
import { useFoundation } from '../features/foundation/useFoundation';

export function App() {
  const { state, retry } = useFoundation();
  const minecraftVersion =
    state.status === 'ready' ? state.snapshot.targetMinecraftVersion : undefined;

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Информация о проекте">
        <a className="brand" href="#main-content" aria-label="Launcher — к содержимому">
          <span className="brand-mark" aria-hidden="true">
            <svg viewBox="0 0 40 40" fill="none">
              <path d="m20 3 16 9v16l-16 9L4 28V12z" stroke="currentColor" strokeWidth="2" />
              <path d="m4 12 16 9 16-9M20 21v16" stroke="currentColor" strokeWidth="2" />
            </svg>
          </span>
          <span>Launcher</span>
        </a>

        <div className="sidebar-section">
          <p className="eyebrow">Рабочее пространство</p>
          <a className="nav-link" href="#main-content" aria-current="page">
            <span className="nav-symbol" aria-hidden="true">
              ⌂
            </span>
            Обзор
          </a>
        </div>

        <div className="sidebar-footer">
          <span className="small-label">Этап разработки</span>
          <strong>01 / Основа проекта</strong>
          <p>Java Edition</p>
        </div>
      </aside>

      <main className="main-content" id="main-content">
        <header className="page-header">
          <div>
            <p className="eyebrow">Minecraft Java Edition</p>
            <h1>Ваше пространство для игры</h1>
          </div>
          <span className="phase-badge">Foundation</span>
        </header>

        <section className="welcome-card" aria-labelledby="welcome-title">
          <div className="welcome-copy">
            <span className="release-label">Первый этап</span>
            <h2 id="welcome-title">Начало нового мира.</h2>
            <p>
              Основа лаунчера: desktop-оболочка, интерфейс и независимое игровое ядро. Установка и
              запуск появятся на следующих этапах.
            </p>
            <div className="play-area">
              <button
                className="button button--primary"
                type="button"
                disabled
                aria-describedby="play-help"
              >
                <span aria-hidden="true">▶</span> Играть
              </button>
              <p id="play-help">Запуск игры ещё не реализован</p>
            </div>
          </div>
          <div className="world-art" aria-hidden="true">
            <span className="world-line world-line--one" />
            <span className="world-line world-line--two" />
            <div className="voxel voxel--back">
              <i />
              <b />
              <em />
            </div>
            <div className="voxel voxel--front">
              <i />
              <b />
              <em />
            </div>
            <div className="voxel voxel--small">
              <i />
              <b />
              <em />
            </div>
            <span className="art-coordinate">JAVA / {minecraftVersion ?? 'VANILLA'}</span>
          </div>
        </section>

        <div className="overview-grid">
          <FoundationStatus state={state} retry={retry} />
          <section className="next-card" aria-labelledby="next-title">
            <p className="eyebrow">Дальше по плану</p>
            <h2 id="next-title">Профили и настройки</h2>
            <p className="muted">
              Отдельная игровая папка, сохранение конфигурации и безопасная работа с файлами.
            </p>
            <div className="next-step">
              <span>02</span>
              <strong>Storage & Profiles</strong>
            </div>
          </section>
        </div>

        <footer className="page-footer">
          <span>Статус функций отображается без демонстрационных данных.</span>
          <span>
            {minecraftVersion ? `Целевая версия ${minecraftVersion}` : 'Minecraft Java Edition'}
          </span>
        </footer>
      </main>
    </div>
  );
}
