/**
 * Groups drag and drop functionality
 * - Desktop drag and drop
 * - Mobile touch drag with long-press
 * - Feed/group reordering
 */

// htmx is declared globally in htmx.d.ts

interface DragState {
  element: HTMLElement | null;
  type: string | null;
  id: string | null;
}

interface TouchDragState {
  active: boolean;
  startTimer: ReturnType<typeof setTimeout> | null;
  clone: HTMLElement | null;
  startX: number;
  startY: number;
  currentDropTarget: HTMLElement | null;
}

const LONG_PRESS_DURATION = 300;
const MOVE_THRESHOLD = 10;

let dragState: DragState = {
  element: null,
  type: null,
  id: null
};

let touchState: TouchDragState = {
  active: false,
  startTimer: null,
  clone: null,
  startX: 0,
  startY: 0,
  currentDropTarget: null
};

// Desktop drag handlers
function setupDraggable(handle: HTMLElement): void {
  handle.setAttribute('draggable', 'true');

  handle.addEventListener('dragstart', (e: DragEvent) => {
    dragState.element = handle;
    dragState.type = handle.dataset.dragType || null;
    dragState.id = handle.dataset.dragId || null;

    // Find the parent row and add opacity
    const row = handle.closest('.flex');
    if (row) row.classList.add('opacity-50');

    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', dragState.id || '');

      // Set a custom drag image
      const dragImage = row || handle;
      e.dataTransfer.setDragImage(dragImage, 20, 20);
    }
  });

  handle.addEventListener('dragend', () => {
    // Remove opacity from parent row
    const row = handle.closest('.flex');
    if (row) row.classList.remove('opacity-50');

    document.querySelectorAll('.drop-highlight').forEach(el =>
      el.classList.remove('drop-highlight')
    );

    dragState = { element: null, type: null, id: null };
  });

  // Touch events for mobile
  handle.addEventListener('touchstart', handleTouchStart, { passive: false });
  handle.addEventListener('touchmove', handleTouchMove, { passive: false });
  handle.addEventListener('touchend', handleTouchEnd);
  handle.addEventListener('touchcancel', handleTouchEnd);
}

function handleTouchStart(this: HTMLElement, e: TouchEvent): void {
  const handle = this;
  const touch = e.touches[0];
  touchState.startX = touch.clientX;
  touchState.startY = touch.clientY;

  // Start long-press timer
  touchState.startTimer = setTimeout(() => {
    startTouchDrag(handle, touch);
  }, LONG_PRESS_DURATION);
}

function handleTouchMove(e: TouchEvent): void {
  const touch = e.touches[0];

  // If drag not active yet, check if moved too much (cancel long-press)
  if (!touchState.active) {
    const dx = Math.abs(touch.clientX - touchState.startX);
    const dy = Math.abs(touch.clientY - touchState.startY);
    if (dx > MOVE_THRESHOLD || dy > MOVE_THRESHOLD) {
      if (touchState.startTimer) {
        clearTimeout(touchState.startTimer);
      }
    }
    return;
  }

  // Drag is active - prevent scrolling and move clone
  e.preventDefault();

  if (touchState.clone) {
    touchState.clone.style.left = (touch.clientX - 20) + 'px';
    touchState.clone.style.top = (touch.clientY - 20) + 'px';
  }

  // Find drop target under finger
  // Temporarily hide clone to get element underneath
  if (touchState.clone) touchState.clone.style.display = 'none';
  const elementUnder = document.elementFromPoint(touch.clientX, touch.clientY);
  if (touchState.clone) touchState.clone.style.display = '';

  // Find the drop target
  const dropTarget = elementUnder?.closest('[data-drop-id]') as HTMLElement | null;

  // Update highlights
  if (touchState.currentDropTarget && touchState.currentDropTarget !== dropTarget) {
    touchState.currentDropTarget.classList.remove('drop-highlight');
  }

  if (dropTarget) {
    // Don't highlight if dropping group on itself
    if (!(dragState.type === 'group' && dropTarget.dataset.dropId === dragState.id)) {
      dropTarget.classList.add('drop-highlight');
      touchState.currentDropTarget = dropTarget;
    }
  } else {
    touchState.currentDropTarget = null;
  }
}

function handleTouchEnd(): void {
  if (touchState.startTimer) {
    clearTimeout(touchState.startTimer);
  }

  if (!touchState.active) {
    return;
  }

  // Perform drop if over a valid target
  if (touchState.currentDropTarget) {
    const targetGroupId = touchState.currentDropTarget.dataset.dropId || '';

    // Don't drop on self
    if (!(dragState.type === 'group' && targetGroupId === dragState.id)) {
      if (dragState.type === 'feed' && dragState.id) {
        htmx.ajax('PUT', '/feeds/' + dragState.id + '/group', {
          target: '#group-list',
          swap: 'innerHTML',
          values: { group_id: targetGroupId }
        });
      } else if (dragState.type === 'group' && dragState.id) {
        htmx.ajax('PUT', '/groups/' + dragState.id + '/parent', {
          target: '#group-list',
          swap: 'innerHTML',
          values: { parent_id: targetGroupId }
        });
      }
    }
    touchState.currentDropTarget.classList.remove('drop-highlight');
  }

  // Cleanup
  endTouchDrag();
}

function startTouchDrag(handle: HTMLElement, touch: Touch): void {
  touchState.active = true;
  dragState.element = handle;
  dragState.type = handle.dataset.dragType || null;
  dragState.id = handle.dataset.dragId || null;

  // Add opacity to source row
  const row = handle.closest('.flex');
  if (row) row.classList.add('opacity-50');

  // Create floating clone
  const rowToClone = row || handle;
  touchState.clone = rowToClone.cloneNode(true) as HTMLElement;
  touchState.clone.classList.add('touch-drag-clone');
  touchState.clone.style.left = (touch.clientX - 20) + 'px';
  touchState.clone.style.top = (touch.clientY - 20) + 'px';
  document.body.appendChild(touchState.clone);

  // Haptic feedback if available
  if (navigator.vibrate) {
    navigator.vibrate(50);
  }
}

function endTouchDrag(): void {
  touchState.active = false;

  // Remove opacity from source row
  if (dragState.element) {
    const row = dragState.element.closest('.flex');
    if (row) row.classList.remove('opacity-50');
  }

  // Remove clone
  if (touchState.clone) {
    touchState.clone.remove();
    touchState.clone = null;
  }

  // Clear highlights
  document.querySelectorAll('.drop-highlight').forEach(el =>
    el.classList.remove('drop-highlight')
  );

  dragState = { element: null, type: null, id: null };
  touchState.currentDropTarget = null;
}

// Desktop drop target handlers
function setupDropTarget(el: HTMLElement): void {
  el.addEventListener('dragover', (e: DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move';
    }

    // Don't allow dropping a group onto itself
    if (dragState.type === 'group' && el.dataset.dropId === dragState.id) {
      return;
    }

    el.classList.add('drop-highlight');
  });

  el.addEventListener('dragleave', (e: DragEvent) => {
    // Only remove highlight if we're actually leaving this element
    if (!el.contains(e.relatedTarget as Node)) {
      el.classList.remove('drop-highlight');
    }
  });

  el.addEventListener('drop', (e: DragEvent) => {
    e.preventDefault();
    el.classList.remove('drop-highlight');

    const targetGroupId = el.dataset.dropId || '';

    // Don't drop on self
    if (dragState.type === 'group' && targetGroupId === dragState.id) {
      return;
    }

    // Make the appropriate HTMX request
    if (dragState.type === 'feed' && dragState.id) {
      htmx.ajax('PUT', '/feeds/' + dragState.id + '/group', {
        target: '#group-list',
        swap: 'innerHTML',
        values: { group_id: targetGroupId }
      });
    } else if (dragState.type === 'group' && dragState.id) {
      htmx.ajax('PUT', '/groups/' + dragState.id + '/parent', {
        target: '#group-list',
        swap: 'innerHTML',
        values: { parent_id: targetGroupId }
      });
    }
  });
}

function initDragDrop(): void {
  // Setup draggable handles (only the grip icons)
  document.querySelectorAll<HTMLElement>('.drag-handle[data-drag-type]')
    .forEach(setupDraggable);

  // Setup drop targets (groups and ungrouped area)
  document.querySelectorAll<HTMLElement>('[data-drop-id]')
    .forEach(setupDropTarget);
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', initDragDrop);

// Re-initialize after HTMX swaps
document.body.addEventListener('htmx:afterSwap', (e: Event) => {
  const detail = (e as CustomEvent).detail;
  const target = detail?.target;
  if (target?.id === 'group-list' || target?.closest?.('#group-list')) {
    initDragDrop();
  }
});
