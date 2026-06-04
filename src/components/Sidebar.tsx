import { useState, useEffect } from "react";
import type { CurrentSession, Page } from "../types";
import { cn } from "../lib/cn";
import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
import toast from "react-hot-toast";

function NavIcon({ name }: { name: string }) {
  const icons: Record<string, React.ReactNode> = {
    contacts: <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" /></svg>,
    structures: <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3.75 21h16.5M4.5 21h15V6.75M9.75 21V3.75h4.5V21M3.75 6.75h16.5" /></svg>,
    categories: <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-18 0v6.75a2.25 2.25 0 002.25 2.25h13.5a2.25 2.25 0 002.25-2.25V6.75m-18 0h18" /></svg>,
    reunions: <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5" /></svg>,
    rgpd: <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z" /></svg>,
    stats: <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M7.5 15V6.75m4.5 8.25V3.75m4.5 11.25v-6m-12 11.25h15.75" /></svg>,
    search: <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>,
    admin: <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M10.5 6h9m-9 6h9m-9 6h9M4.5 6h.008v.008H4.5V6zm0 6h.008v.008H4.5V12zm0 6h.008v.008H4.5V18z" /></svg>,
  };
  return <>{icons[name] ?? null}</>;
}

const NAV_ITEMS: { id: Page; label: string }[] = [
  { id: "contacts", label: "Contacts" },
  { id: "structures", label: "Structures" },
  { id: "categories", label: "Catégories" },
  { id: "reunions", label: "Réunions" },
  { id: "rgpd", label: "RGPD" },
  { id: "stats", label: "Statistiques" },
  { id: "search", label: "Recherche avancée" },
  { id: "admin", label: "Administration" },
];

const UPDATE_CHECK_TIMEOUT_MS = 10000;

async function withUpdateCheckTimeout<T>(promise: Promise<T>): Promise<T> {
  let timeoutId: number | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timeoutId = window.setTimeout(() => reject(new Error("update-check-timeout")), UPDATE_CHECK_TIMEOUT_MS);
  });

  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (timeoutId !== undefined) {
      window.clearTimeout(timeoutId);
    }
  }
}

function UpdateSection() {
  const [version, setVersion] = useState("...");
  const [updateAvailable, setUpdateAvailable] = useState<{ version: string; body: string | null } | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    getVersion().then(v => setVersion(v)).catch(() => setVersion("?"));
  }, []);

  const handleCheck = async () => {
    setChecking(true);
    setUpdateAvailable(null);
    try {
      const update = await withUpdateCheckTimeout(check());
      if (update) {
        setUpdateAvailable({ version: update.version, body: update.body ?? null });
        toast.success(`Mise à jour v${update.version} disponible !`, { duration: 6000 });
      } else {
        toast("Aucune mise à jour disponible", { icon: "✅" });
      }
    } catch {
      setUpdateAvailable(null);
      toast.error("Impossible de vérifier les mises à jour");
    } finally {
      setChecking(false);
    }
  };

  const handleInstall = async () => {
    if (!updateAvailable) return;
    setInstalling(true);
    try {
      const update = await withUpdateCheckTimeout(check());
      if (update) {
        await update.downloadAndInstall();
      }
    } catch {
      // silent — if repo is gone, nothing happens
    } finally {
      setInstalling(false);
    }
  };

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">v{version}</span>
        <button onClick={handleCheck} disabled={checking}
          className="text-[10px] text-muted-foreground hover:text-foreground cursor-pointer disabled:opacity-50">
          {checking ? "..." : "Vérifier MAJ"}
        </button>
      </div>
      {updateAvailable && (
        <div className="rounded-lg border border-emerald-200 bg-emerald-50 p-1.5 text-[10px] text-emerald-800">
          v{updateAvailable.version} disponible
          <button onClick={handleInstall} disabled={installing}
            className="ml-1 font-semibold hover:underline cursor-pointer disabled:opacity-50">
            {installing ? "Installation..." : "Installer"}
          </button>
        </div>
      )}
    </div>
  );
}

export function Sidebar({
  activePage,
  onNavigate,
  visiblePages,
  session,
  onLoginClick,
  onChangePasswordClick,
  onLogout,
}: {
  activePage: Page;
  onNavigate: (p: Page) => void;
  visiblePages: Page[];
  session: CurrentSession | null;
  onLoginClick: () => void;
  onChangePasswordClick: () => void;
  onLogout: () => void;
}) {
  const items = NAV_ITEMS.filter((item) => visiblePages.includes(item.id));
  return (
    <aside className="flex h-screen w-60 flex-col border-r bg-card/60 px-3 py-5 max-lg:hidden">
      <div className="mb-6 px-2">
        <div className="flex items-center gap-2">
          <img src="/logo-crvi-light.png" alt="CRVI" className="h-10 w-10 rounded" />
          <h1 className="text-xl font-bold tracking-tight text-primary">CRVI</h1>
        </div>
        <p className="text-xs text-muted-foreground">Gestion Relation Contact</p>
      </div>
      <nav className="flex flex-1 flex-col gap-1 overflow-y-auto min-h-0">
        {items.map((item) => (
          <button key={item.id} onClick={() => onNavigate(item.id)}
            className={cn(
              "flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-all",
              activePage === item.id
                ? "bg-primary/10 text-primary shadow-sm"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
              "cursor-pointer"
            )}>
            <NavIcon name={item.id} />
            {item.label}
          </button>
        ))}
      </nav>
      <div className="shrink-0 border-t pt-3 text-xs text-muted-foreground px-2">
        <div className="mb-3 space-y-2 rounded-xl border bg-background/70 p-2">
          <div>
            <p className="text-[11px] font-semibold text-foreground">{session?.display_name || "Session inconnue"}</p>
            <p className="text-[10px] text-muted-foreground">
              {session?.is_authenticated ? session.username : session?.anonymous_access_enabled ? "Accès public" : "Connexion requise"}
            </p>
            {session?.is_authenticated && session.must_change_password && (
              <p className="mt-1 text-[10px] text-amber-600">Changement de mot de passe recommandé</p>
            )}
          </div>
          {session?.is_authenticated ? (
            <div className="space-y-1">
              <button onClick={onChangePasswordClick} className="w-full rounded-lg border bg-background px-2 py-1 text-[11px] font-medium hover:bg-muted cursor-pointer">
                Changer mon mot de passe
              </button>
              <button onClick={onLogout} className="w-full rounded-lg border bg-background px-2 py-1 text-[11px] font-medium hover:bg-muted cursor-pointer">
                Se déconnecter
              </button>
            </div>
          ) : (
            <button onClick={onLoginClick} className="w-full rounded-lg border bg-background px-2 py-1 text-[11px] font-medium hover:bg-muted cursor-pointer">
              Se connecter
            </button>
          )}
        </div>
        <p>CRVI Verviers</p>
        <UpdateSection />
      </div>
    </aside>
  );
}

export function MobileNav({
  activePage,
  onNavigate,
  onToggleSidebar,
  sidebarOpen,
  visiblePages,
  session,
  onLoginClick,
  onChangePasswordClick,
  onLogout,
}: {
  activePage: Page;
  onNavigate: (p: Page) => void;
  onToggleSidebar: () => void;
  sidebarOpen: boolean;
  visiblePages: Page[];
  session: CurrentSession | null;
  onLoginClick: () => void;
  onChangePasswordClick: () => void;
  onLogout: () => void;
}) {
  const items = NAV_ITEMS.filter((item) => visiblePages.includes(item.id));
  return (
    <>
      <header className="sticky top-0 z-30 flex h-14 items-center gap-3 border-b bg-background/95 px-4 backdrop-blur lg:hidden">
        <button onClick={onToggleSidebar} className="rounded-lg p-1.5 hover:bg-muted cursor-pointer" aria-label="Menu">
          <svg className="size-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={sidebarOpen ? "M6 18L18 6M6 6l12 12" : "M4 6h16M4 12h16M4 18h16"} />
          </svg>
        </button>
        <h1 className="text-lg font-bold text-primary">CRVI</h1>
      </header>
      {sidebarOpen && (
        <>
          <div className="fixed inset-0 z-40 bg-black/30 lg:hidden" onClick={onToggleSidebar} />
          <aside className="fixed inset-y-0 left-0 z-50 w-64 border-r bg-card p-4 shadow-xl animate-in slide-in-from-left duration-200 lg:hidden">
            <div className="mb-6 flex items-center justify-between">
              <div>
                <h1 className="text-xl font-bold text-primary">CRVI</h1>
                <p className="text-xs text-muted-foreground">Gestion Relation Contact</p>
              </div>
              <button onClick={onToggleSidebar} className="rounded-lg p-1.5 hover:bg-muted cursor-pointer">
                <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
              </button>
            </div>
            <nav className="flex flex-col gap-1.5">
              {items.map((item) => (
                <button key={item.id} onClick={() => { onNavigate(item.id); onToggleSidebar(); }}
                  className={`flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-all cursor-pointer ${
                    activePage === item.id ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-muted"
                  }`}>
                  <NavIcon name={item.id} />
                  {item.label}
                </button>
              ))}
            </nav>
            <div className="mt-4 border-t pt-3">
              <p className="text-xs font-medium text-foreground">{session?.display_name || "Session inconnue"}</p>
              <p className="mb-2 text-[11px] text-muted-foreground">
                {session?.is_authenticated ? session.username : session?.anonymous_access_enabled ? "Accès public" : "Connexion requise"}
              </p>
              {session?.is_authenticated && session.must_change_password && (
                <p className="mb-2 text-[10px] text-amber-600">Changement de mot de passe recommandé</p>
              )}
              {session?.is_authenticated ? (
                <div className="space-y-2">
                  <button
                    onClick={() => { onChangePasswordClick(); onToggleSidebar(); }}
                    className="w-full rounded-lg border bg-background px-3 py-2 text-sm font-medium hover:bg-muted cursor-pointer"
                  >
                    Changer mon mot de passe
                  </button>
                  <button
                    onClick={() => { onLogout(); onToggleSidebar(); }}
                    className="w-full rounded-lg border bg-background px-3 py-2 text-sm font-medium hover:bg-muted cursor-pointer"
                  >
                    Se déconnecter
                  </button>
                </div>
              ) : (
                <button
                  onClick={() => { onLoginClick(); onToggleSidebar(); }}
                  className="w-full rounded-lg border bg-background px-3 py-2 text-sm font-medium hover:bg-muted cursor-pointer"
                >
                  Se connecter
                </button>
              )}
            </div>
          </aside>
        </>
      )}
    </>
  );
}
