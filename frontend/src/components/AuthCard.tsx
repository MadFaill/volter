import { useState } from "react";
import { api, type Admin } from "../api";

// Единственный экран без сессии (ui-concept.md §7.9). Светлая центрированная карточка.
// mode=setup на первый запуск (создать администратора), иначе вход.
export function AuthCard({
  mode,
  onDone,
}: {
  mode: "login" | "setup";
  onDone: (admin: Admin) => void;
}) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const isSetup = mode === "setup";

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const admin = isSetup
        ? await api.setupComplete(username, password)
        : await api.login(username, password);
      onDone(admin);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Что-то пошло не так");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-paper px-4">
      <form
        onSubmit={submit}
        className="w-full max-w-[320px] rounded-md border border-line bg-surface p-6 shadow-card"
      >
        <div className="mb-5 flex items-baseline gap-1">
          <span className="text-title font-semibold tracking-tight">VOLTER</span>
          <span className="text-volt font-semibold">⚡</span>
        </div>

        {isSetup && (
          <p className="mb-4 text-small text-muted">
            Первый запуск. Создайте администратора.
          </p>
        )}

        <Field label="Логин">
          <input
            className={inputClass}
            value={username}
            autoFocus
            autoComplete="username"
            onChange={(e) => setUsername(e.target.value)}
          />
        </Field>

        <Field label="Пароль">
          <input
            className={inputClass}
            type="password"
            value={password}
            autoComplete={isSetup ? "new-password" : "current-password"}
            onChange={(e) => setPassword(e.target.value)}
          />
        </Field>

        {isSetup && (
          <p className="mb-3 text-micro uppercase tracking-wide text-muted">
            Минимум 12 символов
          </p>
        )}

        <button
          type="submit"
          disabled={busy}
          className="mt-1 w-full rounded-sm bg-volt py-2 text-small font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50"
        >
          {isSetup ? "Создать" : "Войти"}
        </button>

        {error && <p className="mt-3 text-small text-fail">{error}</p>}
      </form>
    </div>
  );
}

const inputClass =
  "w-full rounded-sm border border-line bg-surface px-3 py-2 text-body outline-none focus:border-volt focus:ring-2 focus:ring-volt/30";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="mb-3 block">
      <span className="mb-1 block text-micro uppercase tracking-wide text-muted">{label}</span>
      {children}
    </label>
  );
}
