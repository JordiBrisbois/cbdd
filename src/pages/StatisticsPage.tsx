import { useEffect, useMemo, useState } from "react";
import toast from "react-hot-toast";
import type {
  DashboardStats,
  StatsBucket,
  StatsFilters,
  StatsParticipation,
  StatsTopMeeting,
} from "../types";
import { exportWorkbook } from "../lib/export";
import { formatDate } from "../lib/format";
import { invoke } from "../lib/tauri";
import { Icon } from "../lib/ui";

function formatMonthLabel(value: string) {
  const [year, month] = value.split("-");
  if (!year || !month) return value;
  const date = new Date(Number(year), Number(month) - 1, 1);
  return new Intl.DateTimeFormat("fr-BE", { month: "short", year: "numeric" }).format(date);
}

function formatNumber(value: number) {
  return new Intl.NumberFormat("fr-BE").format(value);
}

function formatPercent(value: number) {
  return `${new Intl.NumberFormat("fr-BE", { maximumFractionDigits: 1 }).format(value)} %`;
}

function DashboardCard({
  title,
  value,
  accent,
  hint,
}: {
  title: string;
  value: string;
  accent: string;
  hint?: string;
}) {
  return (
    <div className="rounded-2xl border bg-card p-4 shadow-sm">
      <div className={`mb-3 inline-flex rounded-full px-2.5 py-1 text-[11px] font-semibold uppercase tracking-wide ${accent}`}>
        {title}
      </div>
      <div className="text-3xl font-semibold tracking-tight">{value}</div>
      {hint && <p className="mt-2 text-sm text-muted-foreground">{hint}</p>}
    </div>
  );
}

function BarChartCard({
  title,
  subtitle,
  data,
  tone = "bg-sky-500",
}: {
  title: string;
  subtitle: string;
  data: StatsBucket[];
  tone?: string;
}) {
  const max = Math.max(...data.map((item) => item.value), 1);
  return (
    <div className="rounded-2xl border bg-card p-4 shadow-sm">
      <div className="mb-4">
        <h3 className="text-base font-semibold">{title}</h3>
        <p className="text-sm text-muted-foreground">{subtitle}</p>
      </div>
      <div className="space-y-3">
        {data.length > 0 ? data.map((item) => (
          <div key={`${item.key}-${item.label}`} className="space-y-1.5">
            <div className="flex items-center justify-between gap-3 text-sm">
              <span className="truncate text-muted-foreground">{item.label}</span>
              <span className="font-medium">{formatNumber(item.value)}</span>
            </div>
            <div className="h-2.5 overflow-hidden rounded-full bg-muted">
              <div className={`h-full rounded-full ${tone}`} style={{ width: `${Math.max((item.value / max) * 100, 5)}%` }} />
            </div>
          </div>
        )) : (
          <p className="text-sm text-muted-foreground">Aucune donnée sur la période.</p>
        )}
      </div>
    </div>
  );
}

function DonutChartCard({
  title,
  subtitle,
  data,
  colors,
}: {
  title: string;
  subtitle: string;
  data: StatsBucket[];
  colors: string[];
}) {
  const total = data.reduce((sum, item) => sum + item.value, 0);
  const segments = data.map((item) => total > 0 ? (item.value / total) * 264 : 0);
  const offsets = segments.map((_, index) =>
    segments.slice(0, index).reduce((sum, segment) => sum + segment, 0)
  );
  return (
    <div className="rounded-2xl border bg-card p-4 shadow-sm">
      <div className="mb-4">
        <h3 className="text-base font-semibold">{title}</h3>
        <p className="text-sm text-muted-foreground">{subtitle}</p>
      </div>
      <div className="flex flex-col gap-4 md:flex-row md:items-center">
        <div className="relative mx-auto size-40 shrink-0">
          <svg viewBox="0 0 120 120" className="size-40 -rotate-90">
            <circle cx="60" cy="60" r="42" fill="none" stroke="hsl(var(--muted))" strokeWidth="16" />
            {data.map((item, index) => {
              const segment = segments[index];
              return (
                <circle
                  key={`${item.key}-${index}`}
                  cx="60"
                  cy="60"
                  r="42"
                  fill="none"
                  stroke={colors[index % colors.length]}
                  strokeWidth="16"
                  strokeDasharray={`${segment} 264`}
                  strokeDashoffset={-offsets[index]}
                  strokeLinecap="round"
                />
              );
            })}
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center text-center">
            <span className="text-3xl font-semibold">{formatNumber(total)}</span>
            <span className="text-xs uppercase tracking-wide text-muted-foreground">total</span>
          </div>
        </div>
        <div className="grid flex-1 gap-2">
          {data.length > 0 ? data.map((item, index) => (
            <div key={`${item.key}-${item.label}`} className="flex items-center justify-between gap-3 rounded-xl border bg-background/70 px-3 py-2">
              <div className="flex items-center gap-2">
                <span className="inline-block size-3 rounded-full" style={{ backgroundColor: colors[index % colors.length] }} />
                <span className="text-sm text-muted-foreground">{item.label}</span>
              </div>
              <span className="text-sm font-medium">{formatNumber(item.value)}</span>
            </div>
          )) : (
            <p className="text-sm text-muted-foreground">Aucune donnée disponible.</p>
          )}
        </div>
      </div>
    </div>
  );
}

function AreaTrendCard({
  title,
  subtitle,
  data,
  stroke = "#2563eb",
  fill = "rgba(37, 99, 235, 0.12)",
}: {
  title: string;
  subtitle: string;
  data: StatsBucket[];
  stroke?: string;
  fill?: string;
}) {
  const points = useMemo(() => {
    if (data.length === 0) return "";
    const max = Math.max(...data.map((item) => item.value), 1);
    return data
      .map((item, index) => {
        const x = data.length === 1 ? 180 : (index / (data.length - 1)) * 360;
        const y = 140 - (item.value / max) * 110;
        return `${x},${y}`;
      })
      .join(" ");
  }, [data]);

  const areaPoints = useMemo(() => {
    if (!points) return "";
    return `0,140 ${points} 360,140`;
  }, [points]);

  return (
    <div className="rounded-2xl border bg-card p-4 shadow-sm">
      <div className="mb-4">
        <h3 className="text-base font-semibold">{title}</h3>
        <p className="text-sm text-muted-foreground">{subtitle}</p>
      </div>
      {data.length > 0 ? (
        <>
          <svg viewBox="0 0 360 160" className="h-40 w-full overflow-visible">
            <line x1="0" y1="140" x2="360" y2="140" stroke="hsl(var(--border))" strokeWidth="1" />
            <polygon points={areaPoints} fill={fill} />
            <polyline points={points} fill="none" stroke={stroke} strokeWidth="3" strokeLinejoin="round" strokeLinecap="round" />
            {points.split(" ").map((point, index) => {
              const [x, y] = point.split(",");
              return <circle key={`${point}-${index}`} cx={x} cy={y} r="4" fill={stroke} />;
            })}
          </svg>
          <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-muted-foreground md:grid-cols-4">
            {data.slice(-4).map((item) => (
              <div key={`${item.key}-legend`} className="rounded-lg border bg-background/70 px-2 py-2">
                <div>{formatMonthLabel(item.label)}</div>
                <div className="mt-1 text-sm font-medium text-foreground">{formatNumber(item.value)}</div>
              </div>
            ))}
          </div>
        </>
      ) : (
        <p className="text-sm text-muted-foreground">Aucune donnée sur la période sélectionnée.</p>
      )}
    </div>
  );
}

function DataListCard({
  title,
  subtitle,
  rows,
}: {
  title: string;
  subtitle: string;
  rows: { label: string; value: string; hint?: string }[];
}) {
  return (
    <div className="rounded-2xl border bg-card p-4 shadow-sm">
      <div className="mb-4">
        <h3 className="text-base font-semibold">{title}</h3>
        <p className="text-sm text-muted-foreground">{subtitle}</p>
      </div>
      <div className="space-y-2">
        {rows.length > 0 ? rows.map((row) => (
          <div key={`${row.label}-${row.value}`} className="rounded-xl border bg-background/70 px-3 py-2.5">
            <div className="flex items-center justify-between gap-3">
              <span className="text-sm text-muted-foreground">{row.label}</span>
              <span className="text-sm font-medium">{row.value}</span>
            </div>
            {row.hint && <p className="mt-1 text-xs text-muted-foreground">{row.hint}</p>}
          </div>
        )) : (
          <p className="text-sm text-muted-foreground">Aucune donnée disponible sur la période.</p>
        )}
      </div>
    </div>
  );
}

export function StatisticsPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [filters, setFilters] = useState<StatsFilters>({ start_date: null, end_date: null });
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    let active = true;
    setLoading(true);
    void invoke<DashboardStats>("get_dashboard_stats", { filters })
      .then((payload) => {
        if (!active) return;
        setStats(payload);
      })
      .catch((error) => {
        if (!active) return;
        toast.error(String(error));
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [filters]);

  const exportDashboard = () => {
    if (!stats) return;
    setExporting(true);
    try {
      const sheets = [
        {
          name: "Synthese",
          headers: ["Indicateur", "Valeur"],
          rows: [
            ["Contacts totaux", String(stats.overview.total_contacts)],
            ["Contacts actifs", String(stats.overview.active_contacts)],
            ["Contacts anonymisés", String(stats.overview.anonymized_contacts)],
            ["Contacts à supprimer", String(stats.overview.contacts_to_delete)],
            ["Structures", String(stats.overview.total_structures)],
            ["Catégories", String(stats.overview.total_categories)],
            ["Affiliations", String(stats.overview.total_affiliations)],
            ["Réunions totales", String(stats.overview.total_reunions)],
            ["Réunions sur la période", String(stats.overview.period_reunions)],
            ["Présences sur la période", String(stats.overview.period_presences)],
            ["Moyenne présences / réunion", stats.overview.average_presences_per_reunion.toFixed(1)],
            ["Taux de consentement RGPD", stats.overview.rgpd_consent_rate.toFixed(1)],
            ["Structures partenaires directes", String(stats.overview.partner_direct_structures)],
            ["Période début", filters.start_date || "Toutes"],
            ["Période fin", filters.end_date || "Toutes"],
          ],
        },
        { name: "Reunions mensuelles", headers: ["Mois", "Volume"], rows: stats.meetings_by_month.map((item) => [formatMonthLabel(item.label), String(item.value)]) },
        { name: "Contacts créés", headers: ["Mois", "Volume"], rows: stats.contacts_created_by_month.map((item) => [formatMonthLabel(item.label), String(item.value)]) },
        { name: "Présences statut", headers: ["Statut", "Volume"], rows: stats.attendance_by_status.map((item) => [item.label, String(item.value)]) },
        { name: "Structures catégorie", headers: ["Catégorie", "Volume"], rows: stats.structures_by_category.map((item) => [item.label, String(item.value)]) },
        { name: "Contacts commune", headers: ["Commune", "Volume"], rows: stats.contacts_by_commune.map((item) => [item.label, String(item.value)]) },
        { name: "Structures commune", headers: ["Commune", "Volume"], rows: stats.structures_by_commune.map((item) => [item.label, String(item.value)]) },
        { name: "Réunions organisme", headers: ["Organisme", "Volume"], rows: stats.meetings_by_organisme.map((item) => [item.label, String(item.value)]) },
        { name: "Statuts comptes", headers: ["Statut", "Volume"], rows: stats.account_statuses.map((item) => [item.label, String(item.value)]) },
        { name: "Qualité", headers: ["Contrôle", "Volume"], rows: stats.quality_checks.map((item) => [item.label, String(item.value)]) },
        {
          name: "Top réunions",
          headers: ["Réunion", "Participants", "Date", "Organisme"],
          rows: stats.top_meetings.map((item) => [item.label, String(item.value), item.date_reunion || "", item.organisme || ""]),
        },
        {
          name: "Structures taux présence",
          headers: ["Structure", "Invitations", "Présences", "Taux (%)"],
          rows: stats.top_structures_presence_rate.map((item) => [item.label, String(item.invitations), String(item.presents), item.presence_rate.toFixed(1)]),
        },
        {
          name: "Structures volume présence",
          headers: ["Structure", "Invitations", "Présences", "Taux (%)"],
          rows: stats.top_structures_presence_volume.map((item) => [item.label, String(item.invitations), String(item.presents), item.presence_rate.toFixed(1)]),
        },
        {
          name: "Personnes présence",
          headers: ["Personne", "Invitations", "Présences", "Taux (%)"],
          rows: stats.top_people_presence.map((item) => [item.label, String(item.invitations), String(item.presents), item.presence_rate.toFixed(1)]),
        },
      ];
      exportWorkbook(sheets, `stats_crvi_${new Date().toISOString().slice(0, 10)}.xlsx`);
      toast.success("Export Excel du dashboard généré");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setExporting(false);
    }
  };

  const resetFilters = () => {
    setFilters({ start_date: null, end_date: null });
  };

  const topMeetingRows = useMemo(
    () =>
      (stats?.top_meetings ?? []).map((item: StatsTopMeeting) => ({
        label: item.label,
        value: `${formatNumber(item.value)} participant(s)`,
        hint: [item.organisme, formatDate(item.date_reunion)].filter(Boolean).join(" • "),
      })),
    [stats],
  );

  const qualityRows = useMemo(
    () =>
      (stats?.quality_checks ?? []).map((item) => ({
        label: item.label,
        value: formatNumber(item.value),
      })),
    [stats],
  );

  const mapParticipationRows = (items: StatsParticipation[]) =>
    items.map((item) => ({
      label: item.label || "Sans libellé",
      value: `${formatNumber(item.presents)} présent(s)`,
      hint: `${formatNumber(item.invitations)} invitation(s) • ${formatPercent(item.presence_rate)}`,
    }));

  const structurePresenceRateRows = useMemo(
    () => mapParticipationRows(stats?.top_structures_presence_rate ?? []),
    [stats],
  );

  const structurePresenceVolumeRows = useMemo(
    () => mapParticipationRows(stats?.top_structures_presence_volume ?? []),
    [stats],
  );

  const peoplePresenceRows = useMemo(
    () => mapParticipationRows(stats?.top_people_presence ?? []),
    [stats],
  );

  return (
    <div className="flex flex-col gap-6">
      <div className="rounded-[28px] border bg-gradient-to-br from-sky-50 via-white to-amber-50 p-6 shadow-sm">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div className="max-w-3xl">
            <div className="mb-3 inline-flex items-center gap-2 rounded-full border border-sky-200 bg-white/80 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-sky-700">
              <span className="inline-block size-2 rounded-full bg-sky-500" />
              Statistiques CRVI
            </div>
            <h1 className="text-3xl font-semibold tracking-tight text-slate-900">Tableau de bord réseau, réunions et qualité de base</h1>
            <p className="mt-2 text-sm leading-6 text-slate-600">
              Une vue synthétique pour suivre l’activité du réseau, la participation aux réunions et la qualité opérationnelle de la base.
            </p>
          </div>

          <div className="grid gap-3 rounded-2xl border bg-white/85 p-4 shadow-sm sm:grid-cols-2 xl:min-w-[420px]">
            <div>
              <label className="text-xs font-medium text-muted-foreground">Début de période</label>
              <input
                type="date"
                value={filters.start_date ?? ""}
                min={stats?.available_start_date ?? undefined}
                max={filters.end_date ?? stats?.available_end_date ?? undefined}
                onChange={(e) => setFilters((prev) => ({ ...prev, start_date: e.target.value || null }))}
                className="mt-1 h-10 w-full rounded-xl border bg-background px-3 text-sm"
              />
            </div>
            <div>
              <label className="text-xs font-medium text-muted-foreground">Fin de période</label>
              <input
                type="date"
                value={filters.end_date ?? ""}
                min={filters.start_date ?? stats?.available_start_date ?? undefined}
                max={stats?.available_end_date ?? undefined}
                onChange={(e) => setFilters((prev) => ({ ...prev, end_date: e.target.value || null }))}
                className="mt-1 h-10 w-full rounded-xl border bg-background px-3 text-sm"
              />
            </div>
            <button onClick={resetFilters} className="rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted cursor-pointer">
              Réinitialiser
            </button>
            <button
              onClick={exportDashboard}
              disabled={!stats || exporting}
              className="flex items-center justify-center gap-2 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 disabled:opacity-60 cursor-pointer"
            >
              <Icon name="download" className="size-4" /> {exporting ? "Export..." : "Exporter Excel"}
            </button>
          </div>
        </div>
      </div>

      {loading && !stats && (
        <div className="rounded-2xl border bg-card px-4 py-12 text-center text-muted-foreground shadow-sm">
          Chargement du dashboard...
        </div>
      )}

      {stats && (
        <>
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <DashboardCard
              title="Contacts actifs"
              value={formatNumber(stats.overview.active_contacts)}
              accent="bg-sky-100 text-sky-700"
              hint={`${formatNumber(stats.overview.total_contacts)} contacts au total`}
            />
            <DashboardCard
              title="Structures"
              value={formatNumber(stats.overview.total_structures)}
              accent="bg-emerald-100 text-emerald-700"
              hint={`${formatNumber(stats.overview.partner_direct_structures)} partenaires directs`}
            />
            <DashboardCard
              title="Réunions période"
              value={formatNumber(stats.overview.period_reunions)}
              accent="bg-amber-100 text-amber-700"
              hint={`${formatNumber(stats.overview.period_presences)} présences sur la période`}
            />
            <DashboardCard
              title="Consentement RGPD"
              value={formatPercent(stats.overview.rgpd_consent_rate)}
              accent="bg-rose-100 text-rose-700"
              hint={`${formatNumber(stats.overview.contacts_to_delete)} contacts à traiter`}
            />
          </div>

          <div className="grid gap-4 xl:grid-cols-[1.4fr_1fr]">
            <AreaTrendCard
              title="Réunions par mois"
              subtitle="Évolution de l’activité sur la période sélectionnée"
              data={stats.meetings_by_month}
              stroke="#0284c7"
              fill="rgba(2, 132, 199, 0.16)"
            />
            <DonutChartCard
              title="Présences par statut"
              subtitle="Répartition des statuts de participation"
              data={stats.attendance_by_status}
              colors={["#2563eb", "#f59e0b", "#ef4444", "#10b981", "#8b5cf6"]}
            />
          </div>

          <div className="grid gap-4 xl:grid-cols-2">
            <BarChartCard
              title="Structures par catégorie"
              subtitle="Vision du réseau par catégorie d’organisme"
              data={stats.structures_by_category.slice(0, 8)}
              tone="bg-emerald-500"
            />
            <AreaTrendCard
              title="Création de contacts"
              subtitle="Nouvelles fiches créées par mois"
              data={stats.contacts_created_by_month}
              stroke="#16a34a"
              fill="rgba(22, 163, 74, 0.14)"
            />
          </div>

          <div className="grid gap-4 xl:grid-cols-3">
            <BarChartCard
              title="Top communes - contacts"
              subtitle="Communes les plus représentées dans les personnes"
              data={stats.contacts_by_commune}
              tone="bg-violet-500"
            />
            <BarChartCard
              title="Top communes - structures"
              subtitle="Communes les plus représentées dans les structures"
              data={stats.structures_by_commune}
              tone="bg-cyan-500"
            />
            <BarChartCard
              title="Réunions par organisme"
              subtitle="Organismes les plus actifs sur la période"
              data={stats.meetings_by_organisme}
              tone="bg-amber-500"
            />
          </div>

          <div className="grid gap-4 xl:grid-cols-[1fr_1fr_1.2fr]">
            <DonutChartCard
              title="Statuts des comptes"
              subtitle="Répartition des personnes par statut"
              data={stats.account_statuses}
              colors={["#0f766e", "#f97316", "#64748b", "#dc2626", "#2563eb"]}
            />
            <DataListCard
              title="Qualité de la base"
              subtitle="Points d’attention pour le nettoyage des données"
              rows={qualityRows}
            />
            <DataListCard
              title="Top réunions"
              subtitle="Réunions les plus fréquentées sur la période"
              rows={topMeetingRows}
            />
          </div>

          <div className="grid gap-4 xl:grid-cols-3">
            <DataListCard
              title="Structures les plus présentes (taux)"
              subtitle="Classement par pourcentage de présence (min. 3 invitations)"
              rows={structurePresenceRateRows}
            />
            <DataListCard
              title="Structures les plus présentes (volume)"
              subtitle="Classement par nombre total de présences"
              rows={structurePresenceVolumeRows}
            />
            <DataListCard
              title="Personnes les plus présentes"
              subtitle="Contacts les plus assidus sur la période"
              rows={peoplePresenceRows}
            />
          </div>
        </>
      )}
    </div>
  );
}
