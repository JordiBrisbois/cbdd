import type { Categorie } from "../types";
import { Icon } from "../lib/ui";

export function CategoriesModal({ categories, editingCat, onSave, onDelete, onEdit, onClose }: {
  categories: Categorie[];
  editingCat: { id?: number; nom: string } | null;
  onSave: () => Promise<void>;
  onDelete: (id: number) => Promise<void>;
  onEdit: (cat: { id?: number; nom: string } | null) => void;
  onClose: () => void;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={onClose}>
      <div className="flex w-[95vw] max-w-xl flex-col rounded-2xl border bg-card shadow-xl max-h-[85vh]" onClick={(e) => e.stopPropagation()}>
        <div className="flex shrink-0 items-center justify-between border-b px-6 py-4">
          <h2 className="text-lg font-semibold">Gérer les catégories</h2>
          <button onClick={onClose} className="rounded-lg p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer">
            <svg className="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
          </button>
        </div>
        <div className="overflow-y-auto px-6 py-4">
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-semibold text-muted-foreground">Liste des catégories</span>
              <button onClick={() => onEdit({ nom: "" })}
                className="flex items-center gap-1 rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer">
                <Icon name="plus" className="size-3.5" /> Ajouter
              </button>
            </div>
            {categories.map((c) => (
              <div key={c.id_categorie} className="flex items-center justify-between rounded-lg border bg-muted/30 px-3 py-2">
                <span className="text-sm font-medium">{c.nom_categorie}</span>
                <div className="flex gap-1.5">
                  <button onClick={() => onEdit({ id: c.id_categorie, nom: c.nom_categorie || "" })}
                    className="cursor-pointer text-muted-foreground hover:text-foreground"><Icon name="edit" className="size-4" /></button>
                  <button onClick={() => onDelete(c.id_categorie)}
                    className="cursor-pointer text-red-500 hover:text-red-700"><Icon name="trash" className="size-4" /></button>
                </div>
              </div>
            ))}
          </div>
          {editingCat && (
            <div className="mt-4 rounded-lg border bg-muted/30 p-3">
              <input value={editingCat.nom} onChange={(e) => onEdit({ ...editingCat, nom: e.target.value })} placeholder="Nom de la catégorie" autoFocus
                className="h-10 w-full rounded-lg border bg-background px-3 text-sm focus:border-ring focus:ring-2 focus:ring-ring/30" />
              <div className="mt-3 flex justify-end gap-2">
                <button onClick={() => onEdit(null)} className="cursor-pointer rounded-lg border bg-background px-3 py-1.5 text-xs font-medium hover:bg-muted">Annuler</button>
                <button onClick={onSave} className="cursor-pointer rounded-lg bg-primary px-4 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90">Enregistrer</button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
