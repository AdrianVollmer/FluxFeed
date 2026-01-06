/**
 * Feed filter modal functionality
 * - Modal open/close
 * - Select all/none feeds
 * - Apply filter
 * - Group checkbox cascade
 */

function closeFeedFilterModal(): void {
  const modal = document.getElementById('feed-filter-modal');
  if (modal) {
    modal.innerHTML = '';
  }
}

function selectAllFeeds(): void {
  document.querySelectorAll<HTMLInputElement>('.feed-checkbox, .group-checkbox')
    .forEach(cb => cb.checked = true);
}

function selectNoneFeeds(): void {
  document.querySelectorAll<HTMLInputElement>('.feed-checkbox, .group-checkbox')
    .forEach(cb => cb.checked = false);
}

function applyFeedFilter(): void {
  const feedIds = Array.from(document.querySelectorAll<HTMLInputElement>('.feed-checkbox:checked'))
    .map(cb => cb.value);
  const groupIds = Array.from(document.querySelectorAll<HTMLInputElement>('.group-checkbox:checked'))
    .map(cb => cb.value);

  const params = new URLSearchParams(window.location.search);

  // Clear existing filter params
  params.delete('feed_ids');
  params.delete('group_ids');
  params.delete('offset'); // Reset pagination when filter changes

  if (feedIds.length > 0) {
    params.set('feed_ids', feedIds.join(','));
  }
  if (groupIds.length > 0) {
    params.set('group_ids', groupIds.join(','));
  }

  const queryString = params.toString();
  window.location.href = '/articles' + (queryString ? '?' + queryString : '');
}

function initFeedFilterModal(): void {
  // Toggle group checkbox cascades to all child checkboxes
  document.querySelectorAll<HTMLInputElement>('.group-checkbox').forEach(groupCb => {
    groupCb.addEventListener('change', function() {
      const container = this.closest('.group-container');
      if (container) {
        container.querySelectorAll<HTMLInputElement>('.feed-checkbox, .group-checkbox').forEach(cb => {
          if (cb !== this) {
            cb.checked = this.checked;
          }
        });
      }
    });
  });
}

// Close modal on Escape key
document.addEventListener('keydown', (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    closeFeedFilterModal();
  }
});

// Initialize when modal content is loaded via HTMX
document.body.addEventListener('htmx:afterSwap', (evt: Event) => {
  const detail = (evt as CustomEvent).detail;
  if (detail?.target?.id === 'feed-filter-modal') {
    initFeedFilterModal();
  }
});

// Export for global access
(window as unknown as Record<string, unknown>).closeFeedFilterModal = closeFeedFilterModal;
(window as unknown as Record<string, unknown>).selectAllFeeds = selectAllFeeds;
(window as unknown as Record<string, unknown>).selectNoneFeeds = selectNoneFeeds;
(window as unknown as Record<string, unknown>).applyFeedFilter = applyFeedFilter;
