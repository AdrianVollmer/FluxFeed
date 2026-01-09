/**
 * Article list functionality
 * - View toggle (cards/compact/fullscreen) with cookie persistence
 * - Load more button handling
 * - Article content expand/collapse
 * - Compact row expansion
 * - Fullscreen mode with reader content panel
 */

// htmx is declared globally in htmx.d.ts

// Cookie helper functions
function getCookie(name: string): string | null {
  const match = document.cookie.match(new RegExp('(^| )' + name + '=([^;]+)'));
  return match ? match[2] : null;
}

function setCookie(name: string, value: string, days: number = 365): void {
  const expires = new Date(Date.now() + days * 864e5).toUTCString();
  document.cookie = `${name}=${value}; expires=${expires}; path=/; SameSite=Lax`;
}

interface LoadMoreButton extends HTMLButtonElement {
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
}

// Update load more button target and view parameter
function updateLoadMoreButton(): void {
  const loadMoreBtn = document.getElementById('load-more-btn') as LoadMoreButton | null;
  if (!loadMoreBtn) return;

  const savedView = getCookie('articleView') || 'cards';
  const currentUrl = loadMoreBtn.getAttribute('hx-get');
  if (!currentUrl) return;

  // Update or add view parameter to URL
  const url = new URL(currentUrl, window.location.origin);
  url.searchParams.set('view', savedView);
  loadMoreBtn.setAttribute('hx-get', url.pathname + url.search);

  // Update target based on view
  if (savedView === 'compact' || savedView === 'fullscreen') {
    loadMoreBtn.setAttribute('hx-target', '#articles-compact > div:last-child');
  } else {
    loadMoreBtn.setAttribute('hx-target', '#articles-cards');
  }

  // Reinitialize HTMX for this element
  if (typeof htmx !== 'undefined') {
    htmx.process(loadMoreBtn);
  }
}

// Helper to update button active state
function updateViewButtonState(buttons: NodeListOf<Element>, isActive: boolean): void {
  buttons.forEach(btn => {
    if (isActive) {
      btn.classList.add('bg-blue-600', 'text-white');
      btn.classList.remove('hover:bg-gray-100', 'dark:hover:bg-gray-700');
    } else {
      btn.classList.remove('bg-blue-600', 'text-white');
      btn.classList.add('hover:bg-gray-100', 'dark:hover:bg-gray-700');
    }
  });
}

// View toggle with cookie persistence
function setView(view: string): void {
  // Save preference to cookie (server can read this)
  setCookie('articleView', view);

  // Update UI elements
  const cardsView = document.getElementById('articles-cards');
  const compactView = document.getElementById('articles-compact');
  const fullscreenView = document.getElementById('articles-fullscreen');
  const mainContainer = document.getElementById('articles-container');
  const mainElement = document.querySelector('main');
  const normalLayout = document.getElementById('normal-layout');
  const fullscreenLayout = document.getElementById('fullscreen-layout');

  // All view toggle buttons (desktop sidebar, mobile nav)
  const allCardsBtns = document.querySelectorAll('[id^="view-cards"]');
  const allCompactBtns = document.querySelectorAll('[id^="view-compact"]');
  const allFullscreenBtns = document.querySelectorAll('[id^="view-fullscreen"]');

  // Reset all button states
  updateViewButtonState(allCardsBtns, false);
  updateViewButtonState(allCompactBtns, false);
  updateViewButtonState(allFullscreenBtns, false);

  if (view === 'fullscreen') {
    // Switch to fullscreen layout with modest padding
    normalLayout?.classList.add('hidden');
    fullscreenLayout?.classList.remove('hidden');
    mainContainer?.classList.remove('max-w-6xl', 'mx-auto');
    mainContainer?.classList.add('fullscreen-container');
    // Also expand the main element
    mainElement?.classList.remove('container', 'max-w-7xl');
    mainElement?.classList.add('fullscreen-main');
    updateViewButtonState(allFullscreenBtns, true);
  } else {
    // Switch to normal layout
    normalLayout?.classList.remove('hidden');
    fullscreenLayout?.classList.add('hidden');
    mainContainer?.classList.add('max-w-6xl', 'mx-auto');
    mainContainer?.classList.remove('fullscreen-container');
    // Restore main element
    mainElement?.classList.add('container', 'max-w-7xl');
    mainElement?.classList.remove('fullscreen-main');

    if (view === 'compact') {
      cardsView?.classList.add('hidden');
      compactView?.classList.remove('hidden');
      updateViewButtonState(allCompactBtns, true);
    } else {
      cardsView?.classList.remove('hidden');
      compactView?.classList.add('hidden');
      updateViewButtonState(allCardsBtns, true);
    }
  }

  // Update load more button for new view
  updateLoadMoreButton();
}

// Load article content in fullscreen mode
function loadArticleContent(articleId: number): void {
  const contentPanel = document.getElementById('fullscreen-content');
  if (!contentPanel) return;

  // Show loading state
  contentPanel.innerHTML = `
    <div class="flex items-center justify-center h-full text-gray-500 dark:text-gray-400">
      <div class="text-center">
        <svg class="animate-spin h-8 w-8 mx-auto mb-2" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        Loading article...
      </div>
    </div>
  `;

  // Mark article as read and load content
  if (typeof htmx !== 'undefined') {
    // Mark as read first
    htmx.ajax('POST', `/articles/${articleId}/mark-read-fullscreen`, {
      target: `#article-fullscreen-${articleId}`,
      swap: 'outerHTML'
    });

    // Load reader content
    htmx.ajax('GET', `/articles/${articleId}/reader-content`, {
      target: '#fullscreen-content',
      swap: 'innerHTML'
    });
  }

  // Highlight selected article
  document.querySelectorAll('.fullscreen-article-row').forEach(row => {
    row.classList.remove('bg-blue-50', 'dark:bg-blue-900/20');
  });
  const selectedRow = document.querySelector(`[data-fullscreen-article-id="${articleId}"]`);
  selectedRow?.classList.add('bg-blue-50', 'dark:bg-blue-900/20');
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
  const savedView = getCookie('articleView') || 'cards';
  setView(savedView);
  updateLoadMoreButton();
  checkCollapsibleContent();
});

// Re-apply load more button settings after any swap (handles both main and OOB swaps)
document.body.addEventListener('htmx:afterSwap', () => {
  updateLoadMoreButton();
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

// Toggle expanded content in fullscreen view
function toggleFullscreenExpand(articleId: number): void {
  const expandedEl = document.getElementById(`article-fullscreen-${articleId}-expanded`);
  const rowEl = document.getElementById(`article-fullscreen-${articleId}`);
  const expandBtn = rowEl?.querySelector('.fullscreen-expand-btn svg');

  if (expandedEl) {
    const isHidden = expandedEl.classList.contains('hidden');
    expandedEl.classList.toggle('hidden');

    // Rotate the chevron icon
    if (expandBtn) {
      if (isHidden) {
        expandBtn.classList.add('rotate-180');
      } else {
        expandBtn.classList.remove('rotate-180');
      }
    }
  }
}

// Export for global access
(window as unknown as Record<string, unknown>).setView = setView;
(window as unknown as Record<string, unknown>).toggleFullscreenExpand = toggleFullscreenExpand;
(window as unknown as Record<string, unknown>).checkCollapsibleContent = checkCollapsibleContent;
(window as unknown as Record<string, unknown>).loadArticleContent = loadArticleContent;
