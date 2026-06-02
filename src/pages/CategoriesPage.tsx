import { useState, useEffect, useMemo, useRef } from "react";
import type { Categorie, Personne, PersonneCategorieDetaillee, Structure } from "../types";
import { invoke, formatCivilite, GroupedCell, exportTableFile, formatValuesForMail, splitDelimitedValues } from "../lib/utils";
import { getExportConfig } from "../lib/columns";
import { Icon } from "../lib/ui";
import { DataTable, type TableConfig } from "../components/DataTable";
import { TableExportModal, type TableExportFormat, type TableExportScope } from "../components/TableExportModal";
import { ContactModal } from "../modals/ContactModal";
import { StructureModal } from "../modals/StructureModal";
import { CategoriesModal } from "../modals/CategoriesModal";
import { CopyValuesModal } from "../components/CopyValuesModal";
import toast from "react-hot-toast";
import { useAuth } from "../lib/auth";

export function CategoriesPage() {
  const { can } = useAuth();
  const [categories, setCategories] = useState<Categorie[]>([]);
  const [selectedCat, setSelectedCat] = useState<number | null>(null);
  const [contacts, setContacts] = useState<PersonneCategorieDetaillee[]>([]);
  const [search, setSearch] = useState("");
  const [showCatModal, setShowCatModal] = useState(false);
  const [showCopyModal, setShowCopyModal] = useState(false);
  const [showExportModal, setShowExportModal] = useState(false);
  const [editingCat, setEditingCat] = useState<{ id?: number; nom: string } | null>(null);
  const [contactModal, setContactModal] = useState<Personne | null>(null);
  const [structureModal, setStructureModal] = useState<Structure | null>(null);
  const [loading, setLoading] = useState(false);
  const catFetchIdRef = useRef(0);
  const peopleFetchIdRef = useRef(0);
  const canManageCategories = can("categories.create") || can("categories.update") || can("categories.delete");

  const config: TableConfig = useMemo(() => ({
    id: "categories-contacts",
    columns: [
      { key: "civ", label: "Civ." }, { key: "nom", label: "Nom" }, { key: "prenom", label: "Prénom" },
      { key: "email", label: "Email privé" }, { key: "tel", label: "Tél." }, { key: "adresse", label: "Adresse" },
      { key: "cp", label: "Code postal" }, { key: "commune", label: "Commune" }, { key: "pays", label: "Pays" },
      { key: "statut", label: "Statut" }, { key: "rgpd", label: "RGPD" },
      { key: "structure", label: "Structure(s)" }, { key: "fonction", label: "Fonction(s)" },
      { key: "categorie", label: "Catégorie" }, { key: "emailPro", label: "Email pro" },
      { key: "titre", label: "Titre" }, { key: "telDirect", label: "Tél. direct" }, { key: "gsmPro", label: "GSM pro" },
    ],
    sortAccessors: {
      civ: p => p.civilite as string ?? "", nom: p => p.nom as string ?? "", prenom: p => p.prenom as string ?? "",
      email: p => p.email_prive as string ?? "", tel: p => p.telephone_prive as string ?? "",
      adresse: p => p.adresse_privee as string ?? "", cp: p => p.code_postal_prive as string ?? "",
      commune: p => p.commune_privee as string ?? "", pays: p => p.pays as string ?? "",
      statut: p => p.statut_compte as string ?? "", rgpd: p => (p.consentement_rgpd ? "Oui" : "Non"),
      structure: p => p.structures as string ?? "", fonction: p => p.fonctions as string ?? "",
      categorie: p => p.categories as string ?? "", emailPro: p => p.emails_pro as string ?? "",
      titre: p => p.titres as string ?? "", telDirect: p => p.tels_directs as string ?? "",
      gsmPro: p => p.gsms_pro as string ?? "",
    },
    stickyColumns: { widths: { civ: 80, nom: 140, prenom: 132 } },
  }), []);

  const load = async () => {
    if (!selectedCat) {
      setContacts([]);
      return;
    }

    const id = ++peopleFetchIdRef.current;
    setLoading(true);
    const people = await invoke<PersonneCategorieDetaillee[]>("lister_personnes_categorie_detaillee", { categorieId: selectedCat, recherche: search || undefined }).catch(() => []);
    if (id === peopleFetchIdRef.current) {
      setContacts(people);
      setLoading(false);
    }
  };

  useEffect(() => {
    const id = ++catFetchIdRef.current;
    void invoke<Categorie[]>("lister_categories")
      .then((cats) => {
        if (id === catFetchIdRef.current) setCategories(cats);
      })
      .catch(() => {
        if (id === catFetchIdRef.current) setCategories([]);
      });
  }, []);

  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => {
    if (!selectedCat) {
      setContacts([]);
      setLoading(false);
      return;
    }

    const id = ++peopleFetchIdRef.current;
    setLoading(true);
    void invoke<PersonneCategorieDetaillee[]>("lister_personnes_categorie_detaillee", { categorieId: selectedCat, recherche: search || undefined })
      .then((data) => {
        if (id === peopleFetchIdRef.current) setContacts(data);
      })
      .catch(() => {
        if (id === peopleFetchIdRef.current) setContacts([]);
      })
      .finally(() => {
        if (id === peopleFetchIdRef.current) setLoading(false);
      });
  }, [search, selectedCat]);

  const saveCat = async () => {
    if (!editingCat?.nom.trim()) return;
    await invoke("sauvegarder_categorie", { id: editingCat.id || null, nom: editingCat.nom }).catch(() => {});
    toast.success(editingCat.id ? "Catégorie modifiée" : "Catégorie ajoutée");
    setEditingCat(null);
    setShowCatModal(false);
    const id = ++catFetchIdRef.current;
    void invoke<Categorie[]>("lister_categories")
      .then((cats) => {
        if (id === catFetchIdRef.current) setCategories(cats);
      })
      .catch(() => {});
  };

  const delCat = async (id: number) => {
    if (confirm("Supprimer cette catégorie ?")) {
      await invoke("supprimer_categorie", { id }).catch(() => {});
      toast.success("Catégorie supprimée");
      if (selectedCat === id) setSelectedCat(null);
      const fetchId = ++catFetchIdRef.current;
      void invoke<Categorie[]>("lister_categories")
        .then((cats) => {
          if (fetchId === catFetchIdRef.current) setCategories(cats);
        })
        .catch(() => {});
    }
  };

  const selectedCatName = categories.find(c => c.id_categorie === selectedCat)?.nom_categorie || "";

  const openStructureModal = async (structureId: number) => {
    const s = await invoke<Structure>("get_structure", { id: structureId }).catch(() => null);
    if (s) setStructureModal(s);
  };

  const buildCategoryCopyPayload = (mode: string) => {
    const values = contacts.flatMap((contact) => {
      const privateEmails = splitDelimitedValues(contact.email_prive);
      const proEmails = splitDelimitedValues(contact.emails_pro);

      if (mode === "private") return privateEmails;
      if (mode === "pro") return proEmails;
      return [...privateEmails, ...proEmails];
    });

    return formatValuesForMail(values);
  };

  const mapCategoryContactValue = (contact: PersonneCategorieDetaillee, key: string) => {
    switch (key) {
      case "civ": return contact.type_entree === "structure" ? "Structure" : formatCivilite(contact.civilite);
      case "nom": return contact.type_entree === "structure" ? "Structure sans référent" : (contact.nom || "");
      case "prenom": return contact.prenom || "";
      case "email": return contact.email_prive || "";
      case "tel": return contact.telephone_prive || "";
      case "adresse": return contact.adresse_privee || "";
      case "cp": return contact.code_postal_prive || "";
      case "commune": return contact.commune_privee || "";
      case "pays": return contact.pays || "";
      case "statut": return contact.type_entree === "structure" ? "Structure" : (contact.statut_compte || "");
      case "rgpd": return contact.type_entree === "structure" ? "N/A" : (contact.consentement_rgpd ? "Oui" : "Non");
      case "structure": return contact.structures || "";
      case "fonction": return contact.fonctions || "";
      case "categorie": return contact.categories || "";
      case "emailPro": return contact.emails_pro || "";
      case "titre": return contact.titres || "";
      case "telDirect": return contact.tels_directs || "";
      case "gsmPro": return contact.gsms_pro || "";
      default: return "";
    }
  };

  const exportCategoryContacts = (scope: TableExportScope, format: TableExportFormat) => {
    const exportConfig = scope === "current"
      ? getExportConfig("categories-contacts", config.columns)
      : { keysToExport: config.columns.map((column) => column.key), headers: config.columns.map((column) => column.label) };
    const rows = contacts.map((contact) => exportConfig.keysToExport.map((key) => mapCategoryContactValue(contact, key)));
    exportTableFile(exportConfig.headers, rows, `categories_${selectedCatName}_${new Date().toISOString().slice(0, 10)}`, format);
    toast.success(`${contacts.length} contacts exportés`);
  };

  return (
    <>
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold tracking-tight">Contacts par Catégorie</h1>
          {canManageCategories && (
            <button onClick={() => setShowCatModal(true)}
              className="flex items-center gap-1.5 rounded-lg border bg-background px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors cursor-pointer">
              <Icon name="edit" className="size-3.5" /> Gérer les catégories
            </button>
          )}
        </div>
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex-1 min-w-[200px]">
            <label className="text-xs font-medium text-muted-foreground">Catégorie</label>
            <select value={selectedCat ?? ""} onChange={(e) => setSelectedCat(e.target.value ? Number(e.target.value) : null)}
              className="mt-1 h-10 w-full rounded-xl border bg-background px-4 text-sm focus:border-ring focus:ring-2 focus:ring-ring/30">
              <option value="">— Sélectionner une catégorie —</option>
              {categories.map((c) => <option key={c.id_categorie} value={c.id_categorie}>{c.nom_categorie}</option>)}
            </select>
          </div>
          <div className="flex-1 min-w-[200px]">
            <label className="text-xs font-medium text-muted-foreground">Rechercher</label>
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Nom, prénom, email..."
              className="mt-1 h-10 w-full rounded-xl border bg-background px-4 text-sm placeholder:text-muted-foreground focus:border-ring focus:ring-2 focus:ring-ring/30" />
          </div>
        </div>
      </div>

      {selectedCat && (
        <div className="flex flex-col gap-4 mt-4">
          <div className="flex items-center justify-between rounded-xl border bg-card px-4 py-2.5 shadow-sm">
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">Catégorie :</span>
              <span className="font-medium">{selectedCatName}</span>
              <span className="text-sm text-muted-foreground">({contacts.length} contact(s))</span>
            </div>
            <div className="flex items-center gap-2">
              <button onClick={() => setShowExportModal(true)} className="flex items-center gap-1.5 rounded-xl border bg-background px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted transition-colors cursor-pointer">
                <Icon name="download" className="size-4" /> Exporter
              </button>
              <button onClick={() => setShowCopyModal(true)}
                className="flex items-center gap-1.5 rounded-xl border bg-background px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted transition-colors cursor-pointer">
                <Icon name="mail" className="size-4" /> Copier les emails
              </button>
            </div>
          </div>

          {contacts.length === 0 ? (
            <div className="rounded-xl border bg-card p-8 text-center text-muted-foreground shadow-sm">Aucun contact dans cette catégorie</div>
          ) : (
            <DataTable config={config} data={contacts as unknown as Record<string, unknown>[]} loading={loading}
              renderers={{
                civ: (item) => (item as Record<string, unknown>).type_entree === "structure"
                  ? <span className="text-lg" title="Structure">🏢</span>
                  : formatCivilite(item.civilite as string | null),
                nom: (item) => (item as Record<string, unknown>).type_entree === "structure"
                  ? <span className="italic text-muted-foreground">Structure sans référent</span>
                  : <span className="font-medium">{(item.nom as string) || "—"}</span>,
                prenom: (item) => (item as Record<string, unknown>).type_entree === "structure"
                  ? <span className="text-muted-foreground italic">—</span>
                  : <span>{(item.prenom as string) || "—"}</span>,
                email: (item) => <span className="text-muted-foreground">{(item.email_prive as string) || "—"}</span>,
                tel: (item) => <span className="text-muted-foreground">{(item.telephone_prive as string) || "—"}</span>,
                adresse: (item) => (item.adresse_privee as string) || "—",
                cp: (item) => (item.code_postal_prive as string) || "—",
                commune: (item) => (item.commune_privee as string) || "—",
                pays: (item) => (item.pays as string) || "—",
                statut: (item) => {
                  if ((item as Record<string, unknown>).type_entree === "structure") {
                    return <span className="rounded-full px-2 py-0.5 text-xs font-medium bg-sky-100 text-sky-800">Structure</span>;
                  }
                  const s = item.statut_compte as string | null;
                  return <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${s === "Actif" ? "bg-emerald-100 text-emerald-800" : s === "Anonymisé" ? "bg-gray-200 text-gray-600" : "bg-amber-100 text-amber-800"}`}>{s || "—"}</span>;
                },
                rgpd: (item) => (item as Record<string, unknown>).type_entree === "structure"
                  ? <span className="text-xs text-muted-foreground italic">N/A</span>
                  : <span className={`inline-block size-3 rounded-full ${item.consentement_rgpd ? "bg-emerald-500" : "bg-red-400"}`} />,
                structure: (item) => <GroupedCell value={item.structures as string | null} />,
                fonction: (item) => (item as Record<string, unknown>).type_entree === "structure"
                  ? <span>{(item.fonctions as string) || "—"}</span>
                  : <GroupedCell value={item.fonctions as string | null} />,
                categorie: (item) => <GroupedCell value={item.categories as string | null} max={1} />,
                emailPro: (item) => <span className="text-muted-foreground"><GroupedCell value={item.emails_pro as string | null} /></span>,
                titre: (item) => (item as Record<string, unknown>).type_entree === "structure"
                  ? <span>{(item.titres as string) || "—"}</span>
                  : <GroupedCell value={item.titres as string | null} />,
                telDirect: (item) => <GroupedCell value={item.tels_directs as string | null} />,
                gsmPro: (item) => <GroupedCell value={item.gsms_pro as string | null} />,
              }}
              onRowClick={(item) => {
                if ((item as Record<string, unknown>).type_entree === "structure") {
                  if (can("structures.update")) {
                    const structureId = Math.abs((item as unknown as PersonneCategorieDetaillee).id_personne);
                    openStructureModal(structureId);
                  }
                } else {
                  setContactModal(item as unknown as Personne);
                }
              }}
              header={undefined}
            />
          )}
        </div>
      )}

      {contactModal && <ContactModal personne={contactModal} onClose={() => { setContactModal(null); void load(); }} categories={categories} />}
      {structureModal && <StructureModal structure={structureModal} onClose={() => { setStructureModal(null); void load(); }} categories={categories} />}
      {showCopyModal && (
        <CopyValuesModal
          title="Copier les emails"
          description="Choisis quelles adresses copier pour cette catégorie. Le collage sera formaté en liste unique séparée par des points-virgules."
          options={[
            { id: "private", label: "Emails privés", description: "Copier uniquement les emails privés des contacts affichés." },
            { id: "pro", label: "Emails professionnels", description: "Copier uniquement les emails professionnels des affiliations visibles." },
            { id: "both", label: "Privés + professionnels", description: "Copier l'ensemble des emails privés et professionnels, sans doublons." },
          ]}
          defaultOptionId="pro"
          onResolve={buildCategoryCopyPayload}
          onClose={() => setShowCopyModal(false)}
        />
      )}
      <TableExportModal open={showExportModal} onClose={() => setShowExportModal(false)} onConfirm={exportCategoryContacts} />

      {showCatModal && (
        <CategoriesModal categories={categories} editingCat={editingCat}
          onSave={saveCat} onDelete={delCat}
          onEdit={(cat) => setEditingCat(cat)}
          onClose={() => { setShowCatModal(false); setEditingCat(null); }} />
      )}
    </>
  );
}
