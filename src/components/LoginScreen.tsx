import { useState } from "react";

export function LoginScreen({
  onLogin,
  canContinueAsPublic,
}: {
  onLogin: (username: string, password: string) => Promise<void>;
  canContinueAsPublic: boolean;
}) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);

  const submit = async () => {
    setLoading(true);
    try {
      await onLogin(username, password);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-background">
      <div className="mx-auto flex min-h-screen max-w-5xl items-center px-6 py-10">
        <div className="grid w-full gap-6 lg:grid-cols-[1.1fr_0.9fr]">
          <div className="rounded-[2rem] border bg-card p-8 shadow-sm">
            <div className="mb-8">
              <p className="text-sm font-medium uppercase tracking-[0.24em] text-primary/70">CRVI</p>
              <h1 className="mt-3 text-4xl font-bold tracking-tight text-foreground">Connexion à la base relation contact</h1>
              <p className="mt-3 max-w-xl text-sm leading-6 text-muted-foreground">
                Connecte-toi avec un compte local pour débloquer les actions d'ajout, modification, suppression et l'administration des rôles.
              </p>
            </div>
            <div className="space-y-4">
              <div>
                <label className="text-xs font-medium text-muted-foreground">Email ou identifiant</label>
                <input
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="nom@organisation.be"
                  className="mt-1 h-11 w-full rounded-xl border bg-background px-4 text-sm focus:border-ring focus:ring-2 focus:ring-ring/30"
                />
              </div>
              <div>
                <label className="text-xs font-medium text-muted-foreground">Mot de passe</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="••••••••"
                  className="mt-1 h-11 w-full rounded-xl border bg-background px-4 text-sm focus:border-ring focus:ring-2 focus:ring-ring/30"
                  onKeyDown={(e) => { if (e.key === "Enter") void submit(); }}
                />
              </div>
              <button
                onClick={() => void submit()}
                disabled={loading || !username.trim() || !password.trim()}
                className="h-11 w-full rounded-xl bg-primary px-4 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {loading ? "Connexion..." : "Se connecter"}
              </button>
            </div>
          </div>

          <div className="rounded-[2rem] border bg-gradient-to-br from-primary/10 via-background to-emerald-50 p-8 shadow-sm">
            <h2 className="text-lg font-semibold text-foreground">Accès à l'application</h2>
            <p className="mt-3 text-sm leading-6 text-muted-foreground">
              L'accès à certaines fonctionnalités dépend de votre profil et des autorisations configurées par l'administration.
            </p>
            <div className="mt-6 rounded-2xl border bg-background/80 p-4 text-sm text-muted-foreground">
              <p className="font-medium text-foreground">Connexion sécurisée</p>
              <p className="mt-2 text-sm">
                Utilisez vos identifiants personnels pour accéder à l'application. Si vous ne disposez pas d'un compte ou si vous rencontrez un problème d'accès, rapprochez-vous de la personne responsable de l'administration.
              </p>
            </div>
            <div className="mt-6 rounded-2xl border bg-background/80 p-4">
              <p className="text-sm font-medium text-foreground">
                {canContinueAsPublic ? "Le mode public est actuellement activé." : "Le mode public est désactivé."}
              </p>
              <p className="mt-2 text-sm text-muted-foreground">
                {canContinueAsPublic
                  ? "Certaines informations peuvent rester consultables sans connexion selon la configuration en place."
                  : "Une authentification est requise pour accéder à l'application."}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
