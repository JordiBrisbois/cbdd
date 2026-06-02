import { useState } from "react";
import toast from "react-hot-toast";
import { copyText, Modal } from "../lib/utils";

export interface CopyOption {
  id: string;
  label: string;
  description?: string;
}

export function CopyValuesModal({
  title,
  description,
  options,
  defaultOptionId,
  onResolve,
  onClose,
}: {
  title: string;
  description: string;
  options: CopyOption[];
  defaultOptionId?: string;
  onResolve: (optionId: string) => string;
  onClose: () => void;
}) {
  const [selected, setSelected] = useState(defaultOptionId ?? options[0]?.id ?? "");

  const copy = async () => {
    const text = onResolve(selected).trim();
    if (!text) {
      toast.error("Aucune donnée à copier");
      return;
    }

    try {
      await copyText(text);
      toast.success("Copié dans le presse-papiers");
      onClose();
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <Modal open={true} onClose={onClose} title={title}>
      <div className="space-y-4">
        <p className="text-sm text-muted-foreground">{description}</p>
        <div className="space-y-2">
          {options.map((option) => (
            <label key={option.id} className="flex cursor-pointer items-start gap-3 rounded-xl border bg-background p-3 text-sm">
              <input
                type="radio"
                name="copy-option"
                checked={selected === option.id}
                onChange={() => setSelected(option.id)}
                className="mt-1"
              />
              <span>
                <span className="block font-medium">{option.label}</span>
                {option.description && <span className="block text-xs text-muted-foreground">{option.description}</span>}
              </span>
            </label>
          ))}
        </div>
        <div className="rounded-xl border bg-background px-4 py-3 text-xs text-muted-foreground">
          Format de copie: valeurs uniques séparées par <span className="font-mono text-foreground">;</span>, pratique pour Outlook.
        </div>
        <div className="flex justify-end gap-3">
          <button onClick={onClose} className="rounded-xl border bg-background px-4 py-2 text-sm font-medium hover:bg-muted cursor-pointer">
            Annuler
          </button>
          <button onClick={() => void copy()} className="rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 cursor-pointer">
            Copier
          </button>
        </div>
      </div>
    </Modal>
  );
}
