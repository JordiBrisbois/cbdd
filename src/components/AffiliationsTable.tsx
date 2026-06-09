import type { AffiliationAvecDetails } from "../types";
import { Icon } from "../lib/ui";

export function AffiliationsTable({
  affiliations,
  perspective,
  canEdit,
  canDelete,
  disabled,
  onEdit,
  onDelete,
}: {
  affiliations: AffiliationAvecDetails[];
  perspective: "personne" | "structure";
  canEdit: boolean;
  canDelete: boolean;
  disabled: boolean;
  onEdit: (affiliation: AffiliationAvecDetails) => void;
  onDelete: (affiliation: AffiliationAvecDetails) => void;
}) {
  if (affiliations.length === 0) {
    return <p className="text-sm text-muted-foreground">Aucune affiliation</p>;
  }

  const identityHeaders = perspective === "personne"
    ? ["Structure"]
    : ["Nom", "Prénom"];

  return (
    <div className="overflow-x-auto rounded-lg border">
      <table className="w-full text-xs">
        <thead>
          <tr className="border-b bg-muted/50 text-left">
            {identityHeaders.map((header) => (
              <th key={header} className="px-3 py-2 font-medium text-muted-foreground">{header}</th>
            ))}
            {["Fonction", "Catégorie", "Intitulé", "Email pro", "Tél. fixe pro", "GSM pro"].map((header) => (
              <th key={header} className="px-3 py-2 font-medium text-muted-foreground">{header}</th>
            ))}
            <th className="px-3 py-2" />
          </tr>
        </thead>
        <tbody>
          {affiliations.map((affiliation) => (
            <tr key={affiliation.id_affiliation} className="border-b last:border-0">
              {perspective === "personne" ? (
                <td className="px-3 py-2 font-medium">{affiliation.nom_structure || "—"}</td>
              ) : (
                <>
                  <td className="px-3 py-2 font-medium">{affiliation.nom_personne || "—"}</td>
                  <td className="px-3 py-2">{affiliation.prenom_personne || "—"}</td>
                </>
              )}
              <td className="px-3 py-2">{affiliation.libelle_fonction || "—"}</td>
              <td className="px-3 py-2">{affiliation.nom_categorie || "—"}</td>
              <td className="px-3 py-2">{affiliation.titre_specifique || "—"}</td>
              <td className="px-3 py-2 text-muted-foreground">{affiliation.email_professionnel || "—"}</td>
              <td className="px-3 py-2 text-muted-foreground">{affiliation.telephone_direct || "—"}</td>
              <td className="px-3 py-2 text-muted-foreground">{affiliation.gsm_professionnel || "—"}</td>
              <td className="flex gap-1 px-3 py-2">
                {canEdit && (
                  <button onClick={() => onEdit(affiliation)} disabled={disabled}
                    className="cursor-pointer text-muted-foreground hover:text-foreground disabled:cursor-not-allowed disabled:opacity-60">
                    <Icon name="edit" className="size-4" />
                  </button>
                )}
                {canDelete && (
                  <button onClick={() => onDelete(affiliation)} disabled={disabled}
                    className="cursor-pointer text-red-500 hover:text-red-700 disabled:cursor-not-allowed disabled:opacity-60">
                    <Icon name="trash" className="size-4" />
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
