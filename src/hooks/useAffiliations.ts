import { useCallback } from "react";
import type { AffiliationAvecDetails } from "../types";
import { invoke } from "../lib/tauri";
import { useAsyncData } from "./useAsyncData";

export function useAffiliations(
  owner: { personneId: number } | { structureId: number } | null,
) {
  const loader = useCallback(() => {
    if (!owner) return Promise.resolve([]);
    if ("personneId" in owner) {
      return invoke<AffiliationAvecDetails[]>("lister_affiliations_personne", owner);
    }
    return invoke<AffiliationAvecDetails[]>("lister_affiliations_structure", owner);
  }, [owner]);

  return useAsyncData(loader, [], {
    immediate: !!owner,
    errorMessage: "Impossible de charger les affiliations",
  });
}
