# Extract minor repeated template patterns

A collection of smaller DRY opportunities that can be addressed together.

## 1. Feed color indicator (5 occurrences)

**Files:**
- `src/web/templates/groups/_group_list_content.html` (2x)
- `src/web/templates/articles/_group_tree_item.html` (2x)
- `src/web/templates/articles/feed_filter_modal.html` (1x)

**Pattern:**
```html
<span class="w-3 h-3 rounded-full flex-shrink-0" style="background-color: {{ feed.color }}"></span>
```

**Solution:** Create `_feed_color_dot.html` include. Requires passing color variable.

## 2. Status badge (5 occurrences in logs)

**File:** `src/web/templates/logs/_log_rows.html`

**Pattern:**
```html
<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200">
    {% include "icons/check.html" %}
    Success
</span>
```

**Solution:** The colors vary by status (green/red/yellow), making this harder to extract without conditionals in the include.

## 3. Checkbox list item (3 files)

**Files:**
- `src/web/templates/articles/_group_tree_item.html`
- `src/web/templates/articles/feed_filter_modal.html`

**Pattern:**
```html
<label class="flex items-center gap-2 py-1 px-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded cursor-pointer">
    <input type="checkbox" name="feed_ids" value="{{ feed.id }}"
           class="rounded border-gray-300 dark:border-gray-600 text-blue-600 focus:ring-blue-500">
    <span class="w-3 h-3 rounded-full flex-shrink-0" style="background-color: {{ feed.color }}"></span>
    <span class="text-sm text-gray-700 dark:text-gray-300 truncate">{{ feed.title }}</span>
</label>
```

**Solution:** Could extract if checkbox name and checked condition can be parameterized.

## 4. Load more button (2 files)

**Files:**
- `src/web/templates/articles/_load_more_button.html`
- `src/web/templates/logs/_load_more_button.html`

These are nearly identical but have different HTMX endpoints and query parameters. Could potentially be unified with parameters.

## 5. View toggle buttons

**File:** `src/web/templates/articles/list.html`

The card/compact view toggle is duplicated for desktop (lines 29-37) and mobile (lines 99-106) sections. Could extract to a single include used in both places.
