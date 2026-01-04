# Bug: "load more" in compact mode

When clicking on "Load more" in `/articles` while in compact mode, the
button disappears.

There is already a mechanism in place that re-adds the button after
clicking it. Try to restructure the code such that we don't always have
to consider several cases.
