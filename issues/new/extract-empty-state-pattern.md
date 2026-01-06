# Extract empty state pattern into reusable component

## Problem

The "no items" empty state display is repeated across 4 list pages with similar structure but varying icons and text.

## Files affected

- `src/web/templates/articles/list.html`
- `src/web/templates/feeds/list.html`
- `src/web/templates/groups/_group_list_content.html`
- `src/web/templates/logs/list.html`

## Current pattern

```html
<div class="card text-center py-12">
    {% include "icons/some-icon.html" %}
    <p class="text-xl text-gray-600 dark:text-gray-400 mb-2">No items yet</p>
    <p class="text-sm text-gray-500">Description or call to action</p>
    <a href="/add" class="btn btn-primary mt-4">Add Item</a>
</div>
```

## Proposed solution

Since the icon, title, description, and action all vary, this may be better handled by:
1. Documenting the pattern for consistency
2. Creating a CSS class for the empty state container styling
3. If Askama supports macros with blocks, use that approach
