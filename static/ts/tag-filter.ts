/**
 * Tag filter modal functionality
 * - Modal open/close
 * - Select all/none tags
 * - Apply filter
 */

function closeTagFilterModal(): void {
  const modal = document.getElementById('tag-filter-modal');
  if (modal) {
    modal.innerHTML = '';
  }
}

function selectAllTags(): void {
  document.querySelectorAll<HTMLInputElement>('.tag-checkbox')
    .forEach(cb => cb.checked = true);
}

function selectNoneTags(): void {
  document.querySelectorAll<HTMLInputElement>('.tag-checkbox')
    .forEach(cb => cb.checked = false);
}

function applyTagFilter(): void {
  const tagIds = Array.from(document.querySelectorAll<HTMLInputElement>('.tag-checkbox:checked'))
    .map(cb => cb.value);

  const params = new URLSearchParams(window.location.search);

  // Clear existing tag filter and reset pagination
  params.delete('tag_ids');
  params.delete('offset');

  if (tagIds.length > 0) {
    params.set('tag_ids', tagIds.join(','));
  }

  const queryString = params.toString();
  window.location.href = '/articles' + (queryString ? '?' + queryString : '');
}

// Close modal on Escape key
document.addEventListener('keydown', (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    closeTagFilterModal();
  }
});

// Export for global access
(window as unknown as Record<string, unknown>).closeTagFilterModal = closeTagFilterModal;
(window as unknown as Record<string, unknown>).selectAllTags = selectAllTags;
(window as unknown as Record<string, unknown>).selectNoneTags = selectNoneTags;
(window as unknown as Record<string, unknown>).applyTagFilter = applyTagFilter;
