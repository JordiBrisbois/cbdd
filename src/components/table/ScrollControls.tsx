export function ScrollIndicators({ canScrollLeft, canScrollRight, scrollLeft, scrollRight }: {
  canScrollLeft: boolean; canScrollRight: boolean; scrollLeft: () => void; scrollRight: () => void;
}) {
  return (
    <div className="flex items-center gap-1">
      <button onClick={canScrollLeft ? scrollLeft : undefined} disabled={!canScrollLeft}
        className="flex items-center justify-center size-7 rounded-lg border bg-background hover:bg-muted cursor-pointer text-muted-foreground disabled:opacity-20 disabled:cursor-default"
        title="Défiler vers la gauche">&#8249;</button>
      <button onClick={canScrollRight ? scrollRight : undefined} disabled={!canScrollRight}
        className="flex items-center justify-center size-7 rounded-lg border bg-background hover:bg-muted cursor-pointer text-muted-foreground disabled:opacity-20 disabled:cursor-default"
        title="Défiler vers la droite">&#8250;</button>
    </div>
  );
}

export function ScrollGradients({ canScrollLeft, canScrollRight }: {
  canScrollLeft: boolean; canScrollRight: boolean;
}) {
  return (
    <>
      {canScrollLeft && <div className="pointer-events-none absolute left-0 top-0 bottom-0 w-10 rounded-l-xl bg-gradient-to-r from-card to-transparent z-[1]" />}
      {canScrollRight && <div className="pointer-events-none absolute right-0 top-0 bottom-0 w-10 rounded-r-xl bg-gradient-to-l from-card to-transparent z-[1]" />}
    </>
  );
}

export function FloatingScrollIndicators({ canScrollLeft, canScrollRight, scrollLeft, scrollRight }: {
  canScrollLeft: boolean; canScrollRight: boolean; scrollLeft: () => void; scrollRight: () => void;
}) {
  return (
    <>
      {canScrollLeft && (
        <button
          onClick={scrollLeft}
          className="absolute left-2 top-1/2 -translate-y-1/2 size-6 flex items-center justify-center rounded-full bg-background/50 backdrop-blur-sm border cursor-pointer hover:bg-muted transition-colors z-20"
          title="Défiler vers la gauche">‹</button>
      )}
      {canScrollRight && (
        <button
          onClick={scrollRight}
          className="absolute right-2 top-1/2 -translate-y-1/2 size-6 flex items-center justify-center rounded-full bg-background/50 backdrop-blur-sm border cursor-pointer hover:bg-muted transition-colors z-20"
          title="Défiler vers la droite">›</button>
      )}
    </>
  );
}

export function FloatingScrollBar({ canScrollLeft, canScrollRight, scrollLeft, scrollRight }: {
  canScrollLeft: boolean; canScrollRight: boolean; scrollLeft: () => void; scrollRight: () => void;
}) {
  if (!canScrollLeft && !canScrollRight) return null;
  return (
    <div className="fixed bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-4 rounded-full bg-background/70 backdrop-blur-sm border shadow-sm px-6 py-2.5 z-50">
      <button onClick={canScrollLeft ? scrollLeft : undefined} disabled={!canScrollLeft}
        className="flex items-center justify-center size-9 rounded-full hover:bg-muted transition-colors cursor-pointer disabled:opacity-20 disabled:cursor-default text-foreground/70 text-lg" title="Défiler vers la gauche">‹</button>
      <span className="text-xs text-muted-foreground select-none">Défiler</span>
      <button onClick={canScrollRight ? scrollRight : undefined} disabled={!canScrollRight}
        className="flex items-center justify-center size-9 rounded-full hover:bg-muted transition-colors cursor-pointer disabled:opacity-20 disabled:cursor-default text-foreground/70 text-lg" title="Défiler vers la droite">›</button>
    </div>
  );
}
