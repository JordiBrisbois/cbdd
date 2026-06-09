import { useState, useMemo, useCallback } from "react";
import type { Categorie, Personne, PersonneRefusBDDPresence } from "../types";
import { invoke } from "../lib/tauri";
import { Icon } from "../lib/ui";
import { DataTable, type TableConfig } from "../components/DataTable";
import toast from "react-hot-toast";
import { useAuth } from "../lib/auth";
import { ContactModal } from "../modals/ContactModal";
import { useAsyncData } from "../hooks/useAsyncData";

export function RGPDPage() {
  const { can } = useAuth();
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [selectedPerson, setSelectedPerson] = useState<Personne | null>(null);
  const canUpdateRgpd = can("rgpd.update");
  const canAnonymize = can("rgpd.anonymize");
  const canBulkAnonymize = can("rgpd.anonymize.bulk");
  const loadRgpd = useCallback(async () => {
    const [items, refusBDD, categories] = await Promise.all([
      invoke<Personne[]>("get_personnes_rgpd"),
      invoke<PersonneRefusBDDPresence[]>("lister_personnes_refus_bdd_presence"),
      invoke<Categorie[]>("lister_categories"),
    ]);
    return { items, refusBDD, categories };
  }, []);
  const { data: rgpdData, loading, reload: load } = useAsyncData(
    loadRgpd,
    { items: [], refusBDD: [], categories: [] } as {
      items: Personne[];
      refusBDD: PersonneRefusBDDPresence[];
      categories: Categorie[];
    },
    { errorMessage: "Impossible de charger les données RGPD" },
  );
  const { items, refusBDD, categories } = rgpdData;

  const refusConfig: TableConfig<PersonneRefusBDDPresence> = useMemo(() => ({
    id: "rgpd-refus",
    columns: [
      { key: "select", label: "" },
      { key: "nom", label: "Nom" }, { key: "prenom", label: "Prénom" },
      { key: "email", label: "Email" }, { key: "tel", label: "Tél." },
      { key: "reunion", label: "Réunion" }, { key: "date", label: "Date" },
    ],
    sortAccessors: {
      select: () => "",
      nom: p => p.nom as string ?? "", prenom: p => p.prenom as string ?? "",
      email: p => p.email_prive as string ?? "", tel: p => p.telephone_prive as string ?? "",
      reunion: p => p.reunion_titre as string ?? "", date: p => p.reunion_date as string ?? "",
    },
    stickyColumns: { widths: { select: 64, nom: 140, prenom: 132 } },
    pageSizes: [50, 100, 250, 500, 1000],
  }), []);

  const itemsConfig: TableConfig<Personne> = useMemo(() => ({
    id: "rgpd-traiter",
    columns: [
      { key: "nom", label: "Nom" }, { key: "prenom", label: "Prénom" },
      { key: "consentement", label: "Consentement" }, { key: "statut", label: "Statut" },
      { key: "actions", label: "Actions" },
    ],
    sortAccessors: {
      nom: p => p.nom as string ?? "", prenom: p => p.prenom as string ?? "",
      consentement: p => p.consentement_rgpd ? "Oui" : "Non", statut: p => p.statut_compte ?? "",
      actions: () => "",
    },
    stickyColumns: { widths: { nom: 140, prenom: 132 } },
    pageSizes: [50, 100, 250, 500, 1000],
  }), []);

  const refusalIds = useMemo(() => new Set(refusBDD.map((item) => item.id_personne)), [refusBDD]);
  const otherItems = useMemo(
    () => items.filter((item) => !refusalIds.has(item.id_personne)),
    [items, refusalIds]
  );

  const anonymiser = async (id: number) => {
    if (!canAnonymize) return;
    if (confirm("Anonymiser cette personne ? Cette action est irréversible.")) {
      try {
        await invoke("anonymiser_personne", { personneId: id });
        toast.success("Personne anonymisée");
        void load().catch(() => {});
      } catch (e) {
        toast.error(String(e));
      }
    }
  };
  const majStatut = async (id: number, statut: string) => {
    if (!canUpdateRgpd) return;
    try {
      await invoke("maj_statut_rgpd", { personneId: id, statut });
      toast.success("Statut mis à jour");
      void load().catch(() => {});
    } catch (e) {
      toast.error(String(e));
    }
  };

  const toggleSelect = (id: number) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const selectAll = () => {
    if (selectedIds.size === refusBDD.length) setSelectedIds(new Set());
    else setSelectedIds(new Set(refusBDD.map(r => r.id_personne)));
  };

  const anonymiserEnMasse = async () => {
    if (!canBulkAnonymize) return;
    if (selectedIds.size === 0) { toast.error("Aucune personne sélectionnée"); return; }
    if (confirm(`Anonymiser ${selectedIds.size} personne(s) ? Cette action est irréversible.`)) {
      try {
        const ids = Array.from(selectedIds);
        const count = await invoke<number>("anonymiser_personnes_en_masse", { personneIds: ids });
        toast.success(`${count} personne(s) anonymisée(s)`);
        setSelectedIds(new Set());
        await load();
      } catch (e) { toast.error(String(e)); }
    }
  };

  const anonymiserToutesLesPersonnesATraiter = async () => {
    if (!canBulkAnonymize) return;
    const ids = otherItems
      .filter((item) => item.id_personne && item.statut_compte !== "Anonymisé")
      .map((item) => item.id_personne);

    if (ids.length === 0) {
      toast.error("Aucune personne à anonymiser");
      return;
    }

    if (confirm(`Anonymiser ${ids.length} personne(s) à traiter ? Cette action supprimera les données personnelles et les affiliations, tout en gardant l'historique de présence sous forme anonyme.`)) {
      try {
        const count = await invoke<number>("anonymiser_personnes_en_masse", { personneIds: ids });
        toast.success(`${count} personne(s) anonymisée(s)`);
        await load();
      } catch (e) {
        toast.error(String(e));
      }
    }
  };

  const formatDate = (d: string | null): string => {
    if (!d) return "—";
    try { return new Date(d).toLocaleDateString("fr-BE"); } catch { return d; }
  };

  const openPerson = async (id: number) => {
    try {
      const person = await invoke<Personne>("get_personne_rgpd", { id });
      setSelectedPerson(person);
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Gestion RGPD</h1>
          <p className="mt-1 text-sm text-muted-foreground">Gestion du consentement et anonymisation forte des personnes qui ne souhaitent plus figurer dans la base.</p>
        </div>
        <button onClick={() => void load().catch(() => {})} disabled={loading}
          className="flex items-center gap-1.5 rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted cursor-pointer disabled:opacity-50">
          <Icon name="refresh" className="size-4" /> Actualiser
        </button>
      </div>

      {refusBDD.length > 0 && (
        <div className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 shadow-sm">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm font-semibold text-red-800">⚠️ Personnes ayant refusé la BDD</span>
              <span className="text-sm text-red-600">({refusBDD.length})</span>
            </div>
            <div className="flex items-center gap-2">
              {canBulkAnonymize && (
                <button onClick={selectAll}
                  className="cursor-pointer rounded-lg border border-red-200 bg-white px-3 py-1 text-xs font-medium text-red-700 hover:bg-red-100">
                  {selectedIds.size === refusBDD.length ? "Tout désélectionner" : "Tout sélectionner"}
                </button>
              )}
              {canBulkAnonymize && selectedIds.size > 0 && (
                <button onClick={anonymiserEnMasse}
                  className="cursor-pointer rounded-lg bg-red-600 px-3 py-1 text-xs font-medium text-white hover:bg-red-700">
                  Anonymiser ({selectedIds.size})
                </button>
              )}
            </div>
          </div>

          <DataTable config={refusConfig} data={refusBDD} loading={loading}
            renderers={{
              select: (item) => (
                <label className="flex items-center justify-center">
                  <input
                    type="checkbox"
                    checked={selectedIds.has(item.id_personne as number)}
                    onChange={(e) => {
                      e.stopPropagation();
                      toggleSelect(item.id_personne as number);
                    }}
                    onClick={(e) => e.stopPropagation()}
                    disabled={!canBulkAnonymize}
                    className="rounded border-gray-300 text-red-600 focus:ring-red-500"
                  />
                </label>
              ),
              nom: (item) => <span className="font-medium">{(item.nom as string) || "—"}</span>,
              prenom: (item) => <span>{(item.prenom as string) || "—"}</span>,
              email: (item) => <span className="text-muted-foreground">{(item.email_prive as string) || "—"}</span>,
              tel: (item) => <span className="text-muted-foreground">{(item.telephone_prive as string) || "—"}</span>,
              reunion: (item) => (item.reunion_titre as string) || "—",
              date: (item) => formatDate(item.reunion_date as string | null),
            }}
            rowClassName="hover:bg-red-50/50"
            onRowClick={(item) => void openPerson(item.id_personne as number)}
            header={undefined}
          />
        </div>
      )}

      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{refusBDD.length > 0 ? "Autres personnes à traiter" : "Personnes à traiter"}</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {refusBDD.length > 0
              ? "Cette liste complète l’alerte ci-dessus avec les fiches à revoir pour absence de consentement ou statut manuel."
              : "Les fiches ci-dessous demandent une revue ou une anonymisation selon leur situation RGPD."}
          </p>
        </div>
        {canBulkAnonymize && otherItems.length > 0 && (
          <button
            onClick={anonymiserToutesLesPersonnesATraiter}
            className="cursor-pointer rounded-lg bg-red-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-700"
          >
            Anonymiser toute cette liste
          </button>
        )}
      </div>
      <p className="text-sm text-muted-foreground">
        L’anonymisation forte supprime les données personnelles, efface les affiliations et conserve uniquement un historique de présence non nominatif.
      </p>

      <DataTable config={itemsConfig} data={otherItems} loading={loading}
        renderers={{
          nom: (item) => <span className="font-medium">{(item.nom as string) || "—"}</span>,
          prenom: (item) => <span>{(item.prenom as string) || "—"}</span>,
          consentement: (item) => <span className={`font-medium ${item.consentement_rgpd ? "text-emerald-600" : "text-red-500"}`}>{item.consentement_rgpd ? "Oui" : "Non"}</span>,
          statut: (item) => (item.statut_compte as string) || "—",
          actions: (item) => {
            const statut = item.statut_compte as string | null;
            if (statut === "Anonymisé") return <span className="text-muted-foreground text-xs">—</span>;
            return (
              <div className="flex gap-2">
                {canUpdateRgpd && (
                  <button onClick={(e) => { e.stopPropagation(); majStatut(item.id_personne as number, "A supprimer"); }}
                    className="rounded-lg border border-amber-200 bg-amber-50 px-2 py-1 text-xs font-medium text-amber-700 hover:bg-amber-100 cursor-pointer">
                    Marquer "A supprimer"
                  </button>
                )}
                {canAnonymize && (
                  <button onClick={(e) => { e.stopPropagation(); anonymiser(item.id_personne as number); }}
                    className="rounded-lg border border-red-200 bg-red-50 px-2 py-1 text-xs font-medium text-red-700 hover:bg-red-100 cursor-pointer">
                    Anonymiser
                  </button>
                )}
              </div>
            );
          },
        }}
        onRowClick={setSelectedPerson}
        header={undefined}
      />

      {selectedPerson && (
        <ContactModal
          personne={selectedPerson}
          categories={categories}
          onClose={() => {
            setSelectedPerson(null);
            void load().catch(() => {});
          }}
        />
      )}
    </div>
  );
}
