import { useCallback, useEffect, useState } from "react";
import { api, type Admin } from "./api";
import { AuthCard } from "./components/AuthCard";
import { AppShell } from "./pages/AppShell";

type Gate = "loading" | "setup" | "login" | "ready";

export function App() {
  const [gate, setGate] = useState<Gate>("loading");
  const [admin, setAdmin] = useState<Admin | null>(null);

  const refresh = useCallback(async () => {
    try {
      const { needs_setup } = await api.setupStatus();
      if (needs_setup) {
        setGate("setup");
        return;
      }
      try {
        const me = await api.me();
        setAdmin(me);
        setGate("ready");
      } catch {
        setGate("login");
      }
    } catch {
      // API недоступен — показываем логин (там же видна ошибка соединения при попытке).
      setGate("login");
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  if (gate === "loading") {
    return <Centered>Загрузка…</Centered>;
  }

  if (gate === "ready" && admin) {
    return (
      <AppShell
        admin={admin}
        onLogout={async () => {
          await api.logout();
          setAdmin(null);
          setGate("login");
        }}
      />
    );
  }

  return (
    <AuthCard
      mode={gate === "setup" ? "setup" : "login"}
      onDone={(a) => {
        setAdmin(a);
        setGate("ready");
      }}
    />
  );
}

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-svh items-center justify-center text-muted text-small">{children}</div>
  );
}
