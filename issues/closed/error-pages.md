We already had an issue regarding error pages (see
94bea76e46216e66cf1fd6e3831eaf12647b5d87), but I still get just a string
back in the case of CSRF errors. Can we have a mechanism (middleware?) that
catches all errors and renders a nice error page in HTML? (see `error.html`
template)
