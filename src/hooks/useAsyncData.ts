import { useCallback, useEffect, useRef, useState } from "react";
import toast from "react-hot-toast";

export function useAsyncData<T>(
  loader: () => Promise<T>,
  initialValue: T,
  options: { errorMessage?: string; immediate?: boolean } = {},
) {
  const [data, setData] = useState<T>(initialValue);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();
  const requestId = useRef(0);
  const immediate = options.immediate !== false;

  const reload = useCallback(async () => {
    const id = ++requestId.current;
    setLoading(true);
    setError(undefined);
    try {
      const next = await loader();
      if (id === requestId.current) setData(next);
      return next;
    } catch (error) {
      if (id === requestId.current) {
        const message = options.errorMessage ? `${options.errorMessage}: ${String(error)}` : String(error);
        setError(message);
        toast.error(message);
      }
      throw error;
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  }, [loader, options.errorMessage]);

  useEffect(() => {
    if (!immediate) return;
    void reload().catch(() => {});
  }, [immediate, reload]);

  return { data, setData, loading, error, reload };
}
