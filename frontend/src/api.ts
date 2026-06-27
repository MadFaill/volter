// Тонкий клиент control-plane API. Cookie-сессия httpOnly — отправляется браузером
// автоматически (credentials: same-origin), в JS токен недоступен.

export type SetupStatus = { needs_setup: boolean };
export type Admin = { username: string };

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const resp = await fetch(path, {
    credentials: "same-origin",
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  const text = await resp.text();
  const data = text ? JSON.parse(text) : null;
  if (!resp.ok) {
    throw new Error(data?.error ?? `Ошибка ${resp.status}`);
  }
  return data as T;
}

export const api = {
  setupStatus: () => req<SetupStatus>("/api/setup/status"),
  setupComplete: (username: string, password: string) =>
    req<Admin>("/api/setup/complete", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    }),
  login: (username: string, password: string) =>
    req<Admin>("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    }),
  logout: () => req<{ ok: boolean }>("/api/auth/logout", { method: "POST" }),
  me: () => req<Admin>("/api/auth/me"),
};
