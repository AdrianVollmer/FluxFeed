# Extract modal structure into reusable component

## Problem

The modal overlay, container, header with close button pattern is duplicated across 5 files with ~40 lines of boilerplate each.

## Files affected

- `src/web/templates/feeds/form.html`
- `src/web/templates/feeds/import_form.html`
- `src/web/templates/groups/form.html`
- `src/web/templates/groups/assign_feed.html`
- `src/web/templates/articles/feed_filter_modal.html`

## Current pattern

```html
<!-- Modal overlay -->
<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" onclick="...">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 max-w-md w-full mx-4">
        <div class="flex justify-between items-center mb-4">
            <h2 class="text-xl font-bold">Title</h2>
            <button onclick="close..." class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                {% include "icons/close.html" %}
            </button>
        </div>
        <!-- Content varies -->
    </div>
</div>
```

## Proposed solution

Explore options:
1. Askama macros if supported
2. Split into `_modal_start.html` and `_modal_end.html` includes
3. JavaScript-based modal with content injection

The close button class pattern is also repeated and could become a utility class in CSS.
