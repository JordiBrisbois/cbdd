import { useState, useCallback, useEffect } from "react";

export function useTableScroll(containerRef: React.RefObject<HTMLDivElement | null>) {
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);

  const update = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    setCanScrollLeft(el.scrollLeft > 2);
    setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 2);
  }, [containerRef]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    update();
    el.addEventListener("scroll", update);
    window.addEventListener("resize", update);
    const observer = new MutationObserver(update);
    observer.observe(el, { childList: true, subtree: true, attributes: true });
    return () => {
      el.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
      observer.disconnect();
    };
  }, [containerRef, update]);

  const scrollLeft = () => containerRef.current?.scrollBy({ left: -300, behavior: "smooth" });
  const scrollRight = () => containerRef.current?.scrollBy({ left: 300, behavior: "smooth" });

  return { canScrollLeft, canScrollRight, scrollLeft, scrollRight };
}
