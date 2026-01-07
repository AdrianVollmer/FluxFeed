# Extract form field pattern into reusable component

## Problem

The label + input + help text structure is repeated 9+ times across form templates with inconsistent spacing and styling.

## Files affected

- `src/web/templates/articles/search.html` (3 occurrences)
- `src/web/templates/feeds/form.html` (2 occurrences)
- `src/web/templates/feeds/edit_form.html` (5 occurrences)
- `src/web/templates/groups/form.html` (2 occurrences)
- `src/web/templates/groups/assign_feed.html` (1 occurrence)
- `src/web/templates/feeds/import_form.html` (1 occurrence)

## Current pattern

```html
<div class="mb-4">
    <label for="field-id" class="block text-sm font-medium mb-2 dark:text-gray-200">
        Field Name <span class="text-red-500">*</span>
    </label>
    <input type="text" id="field-id" name="field_name" required
           class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg dark:bg-gray-700 dark:text-white">
    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
        Help text description
    </p>
</div>
```

## Proposed solution

This is difficult to extract with pure Askama includes since the input type and attributes vary significantly. Options:
1. Extract just the label pattern and help text pattern separately
2. Use Askama macros if available
3. Standardize CSS classes in Tailwind config and document the pattern
