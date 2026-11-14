/**
 * Digest page functionality
 * - Swipe left on the feed card advances to the next feed, exactly like clicking
 *   the "Next Feed" link. Non-destructive: it never changes read state, so it
 *   needs no confirmation (unlike "Mark Read & Next", which stays a tap + confirm
 *   dialog everywhere, never a bare gesture).
 */

function initDigestSwipe(): void {
  const card = document.getElementById('digest-card');
  const nextLink = document.getElementById('digest-next-link') as HTMLAnchorElement | null;
  if (!card || !nextLink) return;

  let touchStartX = 0;
  let touchStartY = 0;
  const swipeThreshold = 60; // min px to count as a deliberate swipe

  card.addEventListener('touchstart', (e: TouchEvent) => {
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
  }, { passive: true });

  card.addEventListener('touchend', (e: TouchEvent) => {
    const touchEndX = e.changedTouches[0].clientX;
    const touchEndY = e.changedTouches[0].clientY;
    const deltaX = touchEndX - touchStartX;
    const deltaY = Math.abs(touchEndY - touchStartY);

    // Ignore vertical-dominant touches (the user is scrolling, not swiping)
    if (deltaY > Math.abs(deltaX)) return;

    if (deltaX < -swipeThreshold) {
      window.location.href = nextLink.href;
    }
  }, { passive: true });
}

document.addEventListener('DOMContentLoaded', initDigestSwipe);
