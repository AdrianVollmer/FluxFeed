# Feed View

When clicking on a feed in `/feeds`, I get a 404 error.

Instead, we should see the properties of the feed.

While fixing the 404, add these properties to the feed object (requires
DB migration):

- a color
- a list of tags (requires adding a `Tags` page)
- fetch frequency (default "smart", or a positive integer amount of
  hours - the functionality will be implemented later)
