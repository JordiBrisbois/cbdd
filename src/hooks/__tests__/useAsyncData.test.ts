import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useAsyncData } from "../useAsyncData";

const toastError = vi.hoisted(() => vi.fn());
vi.mock("react-hot-toast", () => ({
  default: { error: toastError },
}));

describe("useAsyncData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("charge les données initiales avec immediate:true", async () => {
    const loader = vi.fn().mockResolvedValue("hello");
    const { result } = renderHook(() => useAsyncData(loader, ""));

    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBe("hello");
  });

  it("retourne l'erreur via toast et rethrows", async () => {
    const loader = vi.fn().mockRejectedValue(new Error("échec"));
    const { result } = renderHook(() =>
      useAsyncData(loader, "", { errorMessage: "Oups" }),
    );

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(toastError).toHaveBeenCalledWith("Oups: Error: échec");
    expect(result.current.error).toBe("Oups: Error: échec");
  });

  it("ignore les réponses de requêtes obsolètes", async () => {
    let resolveSlow!: (v: string) => void;
    const slowPromise = new Promise<string>((r) => { resolveSlow = r; });
    const fastPromise = Promise.resolve("rapide");

    const loader = vi
      .fn()
      .mockReturnValueOnce(slowPromise)
      .mockReturnValueOnce(fastPromise);

    const { result } = renderHook(() => useAsyncData(loader, ""));

    // Wait for first call to be in-flight, then reload
    expect(result.current.loading).toBe(true);
    await act(async () => {
      result.current.reload().catch(() => {});
    });

    // Now resolve the slow one — it should be ignored
    await act(async () => {
      resolveSlow("lent");
    });

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBe("rapide");
  });

  it("ne charge pas au montage avec immediate:false", () => {
    const loader = vi.fn().mockResolvedValue("data");
    const { result } = renderHook(() =>
      useAsyncData(loader, "", { immediate: false }),
    );

    expect(loader).not.toHaveBeenCalled();
    expect(result.current.loading).toBe(false);
    expect(result.current.data).toBe("");
  });
});
