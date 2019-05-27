This crate is an experiment in web handling design. I try to explore the
design space guided primarily by my own good sense, establishing the
principles of the design as I go. With this outset, the experiment could go
either way. Let's see what comes out in the end.

Goal
====
It is common to structure web handling in layers where each layer has a
well-defined responsibility. However, it is not common that this separation of
responsibilities is enforced by the language. Instead, each layer can inspect
the entire HTTP request and manipulate the entire HTTP response. This leaves
open the possibility for layers to have unexpected effects outside of their
responsibility area.

In practice this works out well, but every once in a while there is an
unexpected interaction between two unrelated parts of the system. Perhaps one
layer overwrites a response header initially set by another layer. Maybe you
can resolve it by stacking the layers in *just the right* order. These are
symptoms of broken or insufficient abstraction.

**Goal:** The different parts must be orthogonal and composable.

Principles
==========
The request handling is broken into parts. Each part:

 - should handle one _thing_ fully
 - should _only_ handle one thing

So, when one _part_ is done with the _thing_, no other _parts_ should consider
that _thing_ later on. Nor should it be able to! This way we ensure some level
of orthogonality.

Parts and things
================
The above leaves the definitions of _parts_ and _things_ completely open, and
it is not obvious how to define them.

It could be tempting to use HTTP headers to guide the division of _things._ It
is a reasonable starting point, but it breaks down for features that use
multiple headers and for headers that are affected by multiple features.

_Caching_ is a feature that uses many headers. To name some: Cache-Control,
Last-Modified, ETag, If-Match, If-Modified-Since and so on. There is also
Vary, which tells the user agent how other header fields affect the
cacheability of the response. Both _content-negotiation_, via the Accept and
Content-Type headers, and, say, _authtorzation_, via the Cookie header, affect
the Vary header. So whose responsibility is the Vary header?

Instead we must identify orthogonal features and when multiple features affect
the same header, we must factor the implementation such that the core library
can handle the interaction without causing conflict, and the library users can
support each feature without giving a second's thought to the other feature.

Current flow:

 1. Lookup: the path part of the URL
 2. QueryHandler: the "GET query" part, that is, everything after a possible
    `?` in the request URI
 3. CookieHandler: the Cookie header, also possibly trigger "Vary: Cookie" in
    the response
 4. Resource: Declare ETag. Core library handles the mechanism
 5. Resource: Handle HTTP verb and declare possible response types. Core
    library handles content-negotiation with the Accept header and sets Vary:
    Accept appropriately
