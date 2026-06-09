import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TableExportModal } from "../TableExportModal";

const toastError = vi.hoisted(() => vi.fn());
vi.mock("react-hot-toast", () => ({
  default: { error: toastError },
}));

describe("TableExportModal", () => {
  it("ferme uniquement après succès de la promesse", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onConfirm = vi.fn().mockResolvedValue(undefined);

    render(
      <TableExportModal open={true} onClose={onClose} onConfirm={onConfirm} />,
    );

    await user.click(screen.getByText("Exporter"));
    await waitFor(() => expect(onConfirm).toHaveBeenCalledWith("current", "csv"));
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  });

  it("affiche une erreur en cas d'échec de la promesse", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onConfirm = vi.fn().mockRejectedValue(new Error("bug"));

    render(
      <TableExportModal open={true} onClose={onClose} onConfirm={onConfirm} />,
    );

    await user.click(screen.getByText("Exporter"));

    await waitFor(() => {
      expect(toastError).toHaveBeenCalledWith(
        "Impossible d'exporter les données: Error: bug",
      );
    });

    expect(onClose).not.toHaveBeenCalled();
  });
});
