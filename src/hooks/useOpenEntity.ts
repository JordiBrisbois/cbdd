import { useState } from "react";
import toast from "react-hot-toast";
import type { AffiliationAvecDetails, Personne, Reunion, Structure } from "../types";
import { invoke } from "../lib/tauri";
import { useAuth } from "../lib/auth";

export function useOpenEntity() {
  const { can } = useAuth();
  const [selectedPersonne, setSelectedPersonne] = useState<Personne | null>(null);
  const [selectedStructure, setSelectedStructure] = useState<Structure | null>(null);
  const [selectedReunion, setSelectedReunion] = useState<Reunion | null>(null);
  const [selectedAffiliation, setSelectedAffiliation] = useState<AffiliationAvecDetails | null>(null);

  const openResult = async (item: Record<string, unknown>) => {
    const entity = String(item._entity ?? "");
    const id = Number(item._id ?? 0);
    if (!entity || !id) return;

    try {
      if (entity === "p" && can("personnes.read")) {
        const personne = await invoke<Personne>("get_personne", { id });
        setSelectedPersonne(personne);
        return;
      }
      if (entity === "s" && can("structures.read")) {
        const structure = await invoke<Structure>("get_structure", { id });
        setSelectedStructure(structure);
        return;
      }
      if (entity === "r" && can("reunions.read")) {
        const reunion = await invoke<Reunion>("get_reunion", { id });
        setSelectedReunion(reunion);
        return;
      }
      if (entity === "a" && can("affiliations.read")) {
        const affiliation = await invoke<AffiliationAvecDetails>("get_affiliation", { id });
        setSelectedAffiliation(affiliation);
        return;
      }
      if (entity === "pr" && can("reunions.read")) {
        const reunionId = Number(item._reunion_id ?? 0);
        if (reunionId) {
          const reunion = await invoke<Reunion>("get_reunion", { id: reunionId });
          setSelectedReunion(reunion);
        }
      }
    } catch (error) {
      toast.error(String(error));
    }
  };

  return {
    selectedPersonne, setSelectedPersonne,
    selectedStructure, setSelectedStructure,
    selectedReunion, setSelectedReunion,
    selectedAffiliation, setSelectedAffiliation,
    openResult,
  };
}
