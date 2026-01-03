# Content clipping

When displaying the content of an RSS item in "Cards" mode, it is clipped at a
certain length. This can become a problem with HTML if we remove closing tags or
quotes. We should insert the entire content into the HTML, but hide parts of it
by using CSS for long entries. The user should be able to expand the entire
thing.
