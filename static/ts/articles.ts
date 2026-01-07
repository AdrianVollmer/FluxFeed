/**
 * Article list functionality
 * - View toggle (cards/compact) with localStorage persistence
 * - Load more button handling
 * - Article content expand/collapse
 * - Compact row expansion
 */

// htmx is declared globally in htmx.d.ts

interface LoadMoreButton extends HTMLButtonElement {
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
}

// Update load more button target and view parameter
function updateLoadMoreButton(): void {
  const loadMoreBtn = document.getElementById('load-more-btn') as LoadMoreButton | null;
  if (!loadMoreBtn) return;

  const savedView = localStorage.getItem('articleView') || 'cards';
  const currentUrl = loadMoreBtn.getAttribute('hx-get');
  if (!currentUrl) return;

  // Update or add view parameter to URL
  const url = new URL(currentUrl, window.location.origin);
  url.searchParams.set('view', savedView);
  loadMoreBtn.setAttribute('hx-get', url.pathname + url.search);

  // Update target based on view
  if (savedView === 'compact') {
    loadMoreBtn.setAttribute('hx-target', '#articles-compact > div:last-child');
  } else {
    loadMoreBtn.setAttribute('hx-target', '#articles-cards');
  }

  // Reinitialize HTMX for this element
  if (typeof htmx !== 'undefined') {
    htmx.process(loadMoreBtn);
  }
}

// View toggle with localStorage persistence
function setView(view: string): void {
  // Save preference
  localStorage.setItem('articleView', view);

  // Update UI
  const cardsView = document.getElementById('articles-cards');
  const compactView = document.getElementById('articles-compact');

  // All view toggle buttons (desktop sidebar, mobile nav)
  const allCardsBtns = document.querySelectorAll('[id^="view-cards"]');
  const allCompactBtns = document.querySelectorAll('[id^="view-compact"]');

  if (view === 'compact') {
    cardsView?.classList.add('hidden');
    compactView?.classList.remove('hidden');
    allCardsBtns.forEach(btn => {
      btn.classList.remove('bg-blue-600', 'text-white');
      btn.classList.add('hover:bg-gray-100', 'dark:hover:bg-gray-700');
    });
    allCompactBtns.forEach(btn => {
      btn.classList.add('bg-blue-600', 'text-white');
      btn.classList.remove('hover:bg-gray-100', 'dark:hover:bg-gray-700');
    });
  } else {
    cardsView?.classList.remove('hidden');
    compactView?.classList.add('hidden');
    allCardsBtns.forEach(btn => {
      btn.classList.add('bg-blue-600', 'text-white');
      btn.classList.remove('hover:bg-gray-100', 'dark:hover:bg-gray-700');
    });
    allCompactBtns.forEach(btn => {
      btn.classList.remove('bg-blue-600', 'text-white');
      btn.classList.add('hover:bg-gray-100', 'dark:hover:bg-gray-700');
    });
  }

  // Update load more button for new view
  updateLoadMoreButton();
}

// Check if article content should be collapsed based on height
function checkCollapsibleContent(): void {
  const contentElements = document.querySelectorAll('.article-content');
  const maxHeight = 96; // 6rem = 96px

  contentElements.forEach((content) => {
    // Skip if already expanded by user
    if (content.classList.contains('expanded')) {
      return;
    }

    // Check if content overflows the max height
    if (content.scrollHeight > maxHeight) {
      content.classList.add('collapsed');
    } else {
      content.classList.remove('collapsed');
    }
  });
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
  const savedView = localStorage.getItem('articleView') || 'cards';
  setView(savedView);
  updateLoadMoreButton();
  checkCollapsibleContent();
});

// Re-apply load more button settings after OOB swap
document.body.addEventListener('htmx:afterSwap', (evt: Event) => {
  const detail = (evt as CustomEvent).detail;
  if (detail?.target?.id === 'article-list-footer') {
    updateLoadMoreButton();
  }
  // Also check for collapsible content after HTMX swaps
  checkCollapsibleContent();
});

// Content expand/collapse toggle (event delegation for dynamic content)
document.addEventListener('click', (e: MouseEvent) => {
  const target = e.target as HTMLElement;
  if (target.classList.contains('toggle-content')) {
    const wrapper = target.closest('.article-content-wrapper');
    const content = wrapper?.querySelector('.article-content');

    if (content?.classList.contains('collapsed')) {
      content.classList.remove('collapsed');
      content.classList.add('expanded');
      target.textContent = 'Show less';
    } else if (content) {
      content.classList.remove('expanded');
      content.classList.add('collapsed');
      target.textContent = 'Show more';
    }
  }
});

// Compact article row expansion (event delegation for dynamic content)
document.addEventListener('click', (e: MouseEvent) => {
  const target = e.target as HTMLElement;
  const row = target.closest('.compact-article-row') as HTMLElement | null;
  if (row) {
    const articleId = row.dataset.articleId;
    const expandedRow = document.getElementById(`article-compact-${articleId}-expanded`);

    if (expandedRow) {
      // Toggle visibility
      expandedRow.classList.toggle('hidden');
      // Toggle expanded class on main row for icon rotation
      row.classList.toggle('expanded');
    }
  }
});

// Export for global access
(window as unknown as Record<string, unknown>).setView = setView;
(window as unknown as Record<string, unknown>).checkCollapsibleContent = checkCollapsibleContent;
