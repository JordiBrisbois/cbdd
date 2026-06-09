import { lazy, Suspense, useCallback, useEffect, useMemo, useState } from "react";
import toast from "react-hot-toast";
import type { BackupRunResult, CurrentSession, DBStatus, LoginInput, Page } from "./types";
import { invoke } from "./lib/tauri";
import { AuthContext } from "./lib/auth";
import { Sidebar, MobileNav } from "./components/Sidebar";
import { LoginScreen } from "./components/LoginScreen";
import { ChangePasswordModal } from "./components/ChangePasswordModal";

const SearchPage = lazy(() => import("./pages/SearchPage").then(m => ({ default: m.SearchPage })));
const ContactsPage = lazy(() => import("./pages/ContactsPage").then(m => ({ default: m.ContactsPage })));
const StructuresPage = lazy(() => import("./pages/StructuresPage").then(m => ({ default: m.StructuresPage })));
const CategoriesPage = lazy(() => import("./pages/CategoriesPage").then(m => ({ default: m.CategoriesPage })));
const ReunionsPage = lazy(() => import("./pages/ReunionsPage").then(m => ({ default: m.ReunionsPage })));
const RGPDPage = lazy(() => import("./pages/RGPDPage").then(m => ({ default: m.RGPDPage })));
const StatisticsPage = lazy(() => import("./pages/StatisticsPage").then(m => ({ default: m.StatisticsPage })));
const AdminPage = lazy(() => import("./pages/AdminPage").then(m => ({ default: m.AdminPage })));

const PAGE_PERMISSIONS: Record<Page, string[]> = {
  contacts: ["personnes.read"],
  structures: ["structures.read"],
  categories: ["categories.read"],
  reunions: ["reunions.read"],
  rgpd: ["rgpd.read"],
  stats: ["stats.read"],
  search: ["search.read"],
  admin: ["admin.users", "admin.roles", "admin.settings", "admin.exports", "admin.backups"],
};

export default function App() {
  const [page, setPage] = useState<Page>("contacts");
  const [mobileSidebar, setMobileSidebar] = useState(false);
  const [dbStatus, setDbStatus] = useState<DBStatus | null>(null);
  const [session, setSession] = useState<CurrentSession | null>(null);
  const [showLogin, setShowLogin] = useState(false);
  const [showChangePassword, setShowChangePassword] = useState(false);

  const checkDb = useCallback(async () => {
    try { const s = await invoke<DBStatus>("get_db_status"); setDbStatus(s); } catch { setDbStatus(null); }
  }, []);
  const refreshSession = useCallback(async () => {
    try { const s = await invoke<CurrentSession>("get_current_session"); setSession(s); } catch { setSession(null); }
  }, []);
  const pickDb = useCallback(async () => {
    try {
      const selected = await invoke<string | null>("pick_db");
      await checkDb();
      await refreshSession();
      if (selected) {
        toast.success("Base de données connectée");
      }
    } catch (e) { toast.error(String(e)); }
  }, [checkDb, refreshSession]);
  const login = useCallback(async (username: string, password: string) => {
    try {
      const next = await invoke<CurrentSession>("login", { credentials: { username, password } satisfies LoginInput });
      setSession(next);
      setShowLogin(false);
      toast.success("Connexion réussie");
    } catch (e) {
      toast.error(String(e));
    }
  }, []);
  const logout = useCallback(async () => {
    try {
      await invoke("logout");
      await refreshSession();
      toast.success("Déconnecté");
    } catch (e) {
      toast.error(String(e));
    }
  }, [refreshSession]);

  const can = useCallback((permission: string) => {
    if (!session) return false;
    return session.role_codes.includes("ADMIN") || session.permissions.includes(permission);
  }, [session]);

  const visiblePages = useMemo(() => {
    return (Object.keys(PAGE_PERMISSIONS) as Page[]).filter((candidate) =>
      PAGE_PERMISSIONS[candidate].some((permission) => can(permission))
    );
  }, [can]);

  const currentPage = visiblePages.includes(page) ? page : (visiblePages[0] ?? page);

  useEffect(() => {
    void Promise.all([checkDb(), refreshSession()]);
  }, [checkDb, refreshSession]);

  useEffect(() => {
    if (session?.is_authenticated && session.must_change_password) {
      setShowChangePassword(true);
    }
  }, [session?.is_authenticated, session?.must_change_password]);

  useEffect(() => {
    if (!dbStatus?.error) return;
    toast.error(dbStatus.error, { id: "db-status-error" });
  }, [dbStatus?.error]);

  useEffect(() => {
    if (!session || !dbStatus?.connected || !(session.role_codes.includes("ADMIN") || session.permissions.includes("admin.backups"))) {
      return;
    }

    const run = async () => {
      try {
        const result = await invoke<BackupRunResult>("run_auto_backup_check");
        if (result.status === "Created") {
          console.info("[CRVI-GRC] Backup automatique créé:", result.backup?.name);
        }
      } catch (e) {
        console.error("[CRVI-GRC] Backup automatique échoué:", e);
      }
    };

    void run();
    const timer = window.setInterval(() => {
      void run();
    }, 15 * 60 * 1000);

    return () => window.clearInterval(timer);
  }, [dbStatus?.connected, session]);

  const authValue = useMemo(() => ({
    session,
    refreshSession,
    login,
    logout,
    can,
    isAdmin: !!session?.role_codes.includes("ADMIN"),
  }), [can, login, logout, refreshSession, session]);

  if (!session) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="mx-auto mt-10 w-full max-w-xl rounded-2xl border bg-card p-6 shadow-sm">
          <h1 className="text-lg font-semibold">Connexion à la base de données requise</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            Aucune session n&apos;est disponible. Sélectionne la base SQLite pour démarrer l&apos;application.
          </p>
          {dbStatus?.error && (
            <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
              {dbStatus.error}
            </div>
          )}
          <div className="mt-5 flex items-center justify-between rounded-xl border bg-background px-4 py-2.5">
            <span className="text-sm text-muted-foreground">
              {dbStatus?.connected
                ? `Base connectée: ${dbStatus.path?.split("/").pop() || dbStatus.path?.split("\\").pop() || "BDD"}`
                : dbStatus?.path
                  ? `Chemin mémorisé: ${dbStatus.path}`
                  : "Aucune base connectée"}
            </span>
            <button
              onClick={pickDb}
              className="cursor-pointer rounded-lg border bg-background px-3 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
            >
              Choisir une base
            </button>
          </div>
        </div>
      </div>
    );
  }

  const requiresLogin = !session.is_authenticated && !session.anonymous_access_enabled;
  if (requiresLogin) {
    return <LoginScreen onLogin={login} canContinueAsPublic={false} />;
  }

  return (
    <AuthContext.Provider value={authValue}>
      {showLogin && <LoginScreen onLogin={login} canContinueAsPublic={session.anonymous_access_enabled} />}
      {showChangePassword && session?.is_authenticated && (
        <ChangePasswordModal
          required={session.must_change_password}
          onClose={() => setShowChangePassword(false)}
          onChanged={refreshSession}
        />
      )}
      {!showLogin && (
        <div className="flex h-screen overflow-hidden">
          <Sidebar
            activePage={currentPage}
            onNavigate={setPage}
            visiblePages={visiblePages}
            session={session}
            onLoginClick={() => setShowLogin(true)}
            onChangePasswordClick={() => setShowChangePassword(true)}
            onLogout={() => void logout()}
          />
          <div className="flex min-w-0 flex-1 flex-col">
            <MobileNav
              activePage={currentPage}
              onNavigate={setPage}
              onToggleSidebar={() => setMobileSidebar(!mobileSidebar)}
              sidebarOpen={mobileSidebar}
              visiblePages={visiblePages}
              session={session}
              onLoginClick={() => setShowLogin(true)}
              onChangePasswordClick={() => setShowChangePassword(true)}
              onLogout={() => void logout()}
            />
            <main className="flex-1 overflow-y-auto overflow-x-hidden">
              <div className="mx-auto flex w-full max-w-7xl flex-col px-4 py-4 lg:px-8 lg:py-6">
                <div className="mb-4 flex items-center justify-between rounded-xl border bg-card px-4 py-2.5 shadow-sm">
                  <div className="flex items-center gap-3 text-sm">
                    <span className={`inline-block size-2.5 rounded-full ${dbStatus?.connected ? "bg-emerald-500" : "bg-amber-500"}`} />
                    <span className="text-muted-foreground">
                      {dbStatus?.connected
                        ? `Connecté: ${dbStatus.path?.split("/").pop() || dbStatus.path?.split("\\").pop() || "BDD"}`
                        : "Aucune base de données"}
                    </span>
                  </div>
                  <button onClick={pickDb}
                    className="cursor-pointer rounded-lg border bg-background px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors">
                    {dbStatus?.connected ? "Changer de base" : "Choisir une base"}
                  </button>
                </div>
                <Suspense fallback={<div className="flex items-center justify-center p-12 text-sm text-muted-foreground">Chargement…</div>}>
                  {currentPage === "contacts" && <ContactsPage />}
                  {currentPage === "structures" && <StructuresPage />}
                  {currentPage === "categories" && <CategoriesPage />}
                  {currentPage === "reunions" && <ReunionsPage />}
                  {currentPage === "rgpd" && <RGPDPage />}
                  {currentPage === "stats" && <StatisticsPage />}
                  {currentPage === "search" && <SearchPage />}
                  {currentPage === "admin" && <AdminPage />}
                </Suspense>
              </div>
            </main>
          </div>
        </div>
      )}
    </AuthContext.Provider>
  );
}
