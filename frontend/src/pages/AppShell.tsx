import type { Admin } from "../api";

// Каркас приложения за логином (Ш0а). Наполнение — диалоги/контролы — Ш10/Ш11.
export function AppShell({ admin, onLogout }: { admin: Admin; onLogout: () => void }) {
  return (
    <div className="flex min-h-svh flex-col bg-paper">
      <header className="flex items-center justify-between border-b border-line px-4 py-3">
        <div className="flex items-baseline gap-1">
          <span className="text-title font-semibold tracking-tight">VOLTER</span>
          <span className="text-volt font-semibold">⚡</span>
        </div>
        <div className="flex items-center gap-3 text-small text-muted">
          <span className="font-mono text-mono-data">{admin.username}</span>
          <button
            onClick={onLogout}
            className="rounded-sm border border-line px-2 py-1 text-small hover:bg-volt-wash"
          >
            Выйти
          </button>
        </div>
      </header>

      <main className="flex flex-1 items-center justify-center">
        <div className="mx-auto max-w-thread px-6 text-center">
          <p className="text-title">Каркас готов.</p>
          <p className="mt-2 text-small text-muted">
            Доступ закрыт логином. Диалоги, связки и контролы — следующие шаги плана (Ш10–Ш11).
          </p>
        </div>
      </main>
    </div>
  );
}
