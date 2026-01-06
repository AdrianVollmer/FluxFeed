# Refactor templates

Refactor the templates to improve readability. Make it more DRY, like
using `include` for SVG images.

Also, move JavaScript code into dedicated files. In fact, make them
TypeScript so we can lint and typecheck them at compile time. Add some
mechanism such that cached JavaScript files are busted in the user's
browser after an update. Like this:

> A more popular and manageable way is to keep hashes inside the file
> names. Hashes, if you don’t know, are fixed length character
> representations of any content and they are irreversible, meaning you
> can get the hash from the file but you can’t get the file from the
> hash. Hashes are perfect for this, because when a file changes its
> hash will change, so if we keep the hash inside the filename
> `index.\[someHashHere\].js` browsers will detect it and load it
> instead of an old file.

The conversion to typescript should happen in a separate git commit.
