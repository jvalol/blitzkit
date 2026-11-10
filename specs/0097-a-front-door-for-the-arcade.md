# 0097 A front door for the arcade

**Status:** draft
**Date:** 2026-11-09
**Built:** 2026-11-09, and run against a local server. The alias is not added
yet, so nothing answers on the name.

## Goal

A link that puts a stranger inside the building, with nothing to install and
nothing to read first.

The arcade has been playable in a browser since spec 0053's work reached it,
and the only way in is `blitzkit.jva.lol/docs/capriccio/?play=capriccio`: a
documentation page, a sidebar of other pages, a still of the hall, and a
button under it. Everything on that page is addressed to somebody deciding
whether to use the engine. Jake's reason for wanting the other door, in his
words, is "people who don't know how to run cargo, just the lay population."

So: `arcadium.jva.lol`, which is a building and not a crate.

## Behavior

### One page, and it is the arcade

`/arcadium/` on this site, rendered by `layouts/arcadium/single.html`, which
is a whole HTML document rather than a block in the theme's. The docs chrome
is the thing being removed, so inheriting it and hiding it would be the wrong
shape.

Everything a reader sees is in `content/arcadium.md`: the prose in the body,
and the four strings the script swaps in as front matter. The layout holds no
English. That is for Jake's sake rather than the template's: proposed copy is
read and signed in one file instead of being hunted through a script.

### The same Netlify site, under a second name

`arcadium.jva.lol` is a domain alias on the site this repo already deploys,
plus one rewrite:

    from = "https://arcadium.jva.lol/"  to = "/arcadium/"  status = 200

The alternative was a second Netlify site. It would build the same repo to
publish the same 7.9 MB of wasm that `site/scripts/build` already copies in,
and a second build of one artifact is a second thing to keep aligned.

**The rule names the root and nothing else, and the obvious `/*` is a trap.**
Everything on the alias is served by this same site, the arcade's module
included, so the page fetches `arcadium.jva.lol/web/capriccio.js`. A `/*`
rewrite answers that with the page's own HTML, and what the reader gets is a
module error about an unexpected `<`. The module cannot be fetched from the
canonical host instead, because a cross-origin `import()` wants CORS headers
that Netlify does not send by default.

What the alias costs is that every docs page answers on both names. Hugo
writes `<link rel=canonical>` from `baseURL`, so every one of them names
`blitzkit.jva.lol` on both hosts, which is what keeps the alias from becoming
a second copy of the site in search results. A reader who lands on
`arcadium.jva.lol/docs/testing/` gets the docs page under a name nobody linked
it by, which is untidy and does no harm; a redirect for it is a second rule
and a second chance to catch a path that matters.

DNS needs nothing. jva.lol is on Netlify DNS (its nameservers are NS1's), so
adding the alias in the site's settings writes the record.

### Nothing is fetched until somebody asks

The arcade is 7.9 MB, 4.6 MB over the wire. Spec 0053 settled this for twelve
examples on one page and it holds for one building on an empty one: a visitor
who opened the page to see what it was should not pay five megabytes to find
out. The button says the size, and the import happens in its handler.

### What the page has to do, and it is little

Lay out a canvas marked `data-blitzkit` (`src/lib.rs:655`) at the size of the
window. The engine asks the canvas how wide it is laid out every frame and
sizes its surface to the answer, which is spec 0053's conclusion, so a canvas
that fills the window is the whole of the arrangement. No width is passed to
anything.

### Two readers get told instead

**No WebGPU.** `navigator.gpu` absent means there is nothing to draw with and
no fallback is coming (spec 0053, out of scope: WebGL2 has no answer for the
multisampled depth spec 0029 wants). The docs loader prints this into its
button. Here it replaces the button, because this page has nothing else on it.

**A touch screen.** All of it wants WASD and a mouse. The docs loader returns
silently on `(pointer: coarse) and (hover: none)` and lets the clip stand in
its place. This page has no clip to fall back to, so it says what the thing
needs rather than leaving a reader to press a button that would spend five
megabytes on something they cannot move.

### And one thing the page has to admit

The cabinets do not work in a browser. `Cabinet::found` keeps a binary only
when `binary.is_file()` (`capriccio/src/cabinet.rs:32`), a browser has no
files, so `is_built` is false for every cabinet and `start` returns false
before it reaches `Command::new` (`cabinet.rs:75`). The hall is a hall of
screenshots, which is most of what a cabinet is, and pressing Return at one
does nothing at all.

Saying so on the page is the cheap half of the answer. The other half is
making a cabinet in a browser send the reader to that game's own build, which
four of the games have (`starry`, `snake`, `slider`, `poolhall`, offered on
the games page), and that is a change in capriccio rather than in this site.
It is not in this spec.

## Acceptance criteria

Nothing here is a Rust test; this is a page, and the engine's tests cannot see
it. What the repo's own gates do cover:

- The page names nothing the published crate has not got. — `scripts/check-site`
  reads `site/content/*.md`, which includes this page.
- No copy ships while it is still marked. — `scripts/ship-check` reads
  `site/`, and `site/scripts/build` runs it first, so a marked page fails the
  Netlify build rather than reaching the web.

What they do not cover, and it is worth naming: `./check-listings` asks that
every example the site offers to play is built and every build is offered, and
it reads `play name=` out of `site/content/docs/*.md` only. This page offers
the arcade without that shortcode. Nothing breaks today because the capriccio
docs page still offers it, and if that page ever stops, the gate goes quiet
about 7.9 MB rather than failing.

## By hand

Checked on `hugo server` 2026-11-09, in the browser the desktop app carries,
which has WebGPU:

- Nothing out of `/web` is fetched before the button is pressed, and after it
  `capriccio.js` (93 KB) and `capriccio_bg.wasm` (8,091 KB) arrive and nothing
  else does.
- The canvas comes up 1024 by 768 laid out and 2048 by 1536 backed, which is
  the window at this screen's pixel ratio and not spec 0053's one-pixel
  surface, and the hall draws with its marquee lit.
- The keyboard reaches it. Holding the left arrow swings the camera round to
  the side wall. Worth writing down how that was told apart from nothing
  happening: the hall animates on its own, so comparing frames says only that
  the scene is alive. What distinguishes a turn is the whole frame's mean
  colour, which moved by 47 on the docs page and by 17 here and barely at all
  when no key was held.

What a local server cannot answer, and it is the half that can still be
wrong, because both halves are the alias rather than the page:

- `arcadium.jva.lol/` answers with the building, and
  `arcadium.jva.lol/web/capriccio.js` with the module rather than with the
  page, which is the rewrite trap above and reads as a code fault.

## Out of scope

**Making the cabinets play.** Named above. It is capriccio's to answer.

**A second Netlify site, and a repo of its own.** The wasm is built here and
committed here, and that is where it should stay while there is one page
serving it.

**Sound, and `wasm-opt`.** Both spec 0053's, both still true.
