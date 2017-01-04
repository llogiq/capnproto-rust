---
layout: post
title: capnp-rpc-rust now uses futures-rs
author: dwrensha
---

The concurrency story of
[capnp-rpc-rust](https://github.com/dwrensha/capnp-rpc-rust)
gets a major update in today's version 0.8 release.
Previously, capnp-rpc-rust was built
on top of [GJ](https://github.com/dwrensha/gj),
an event loop framework designed specifically for Cap'n Proto,
described in some of my [previous]({{site.baseurl}}/2015/05/25/asynchronous-io-with-promises.html)
[posts]({{site.baseurl}}/2016/01/11/async-rpc.html).
The new version drops GJ in favor of
[futures-rs](https://github.com/alexcrichton/futures-rs),
a library that is quickly becoming the standard
foundation for asynchronous programming in Rust.

At the level of types, the update is fairly
straightforward.
The main asynchronous building block in GJ is the struct
`Promise<T, E>`, representing a `Result<T, E>` that might not
be ready yet. In the world of futures-rs, a `gj::Promise<T,E>` can be described as
roughly equivalent to a `Box<futures::Future<Item=T,Error=E>>`.
Many nice proporties derive from the fact that `Future` is a *trait*, not a struct,
and does not need to be put in a `Box`.
Concrete types implementing `Future` can be used in generics,
making it possible for combinators like `.then()` and `.join()`
to avoid heap allocations
and to not lose any type information.
In particular, the typechecker can know at compile time
whether it is safe to send a future between threads!

The Rust community has a growing ecosystem of libraries based on
futures-rs, and today's capnp-rpc-rust release
should enable easy interoperation with any of them.
To demonstrate, I have implemented a
[simple example](https://github.com/dwrensha/capnp-rpc-rust/tree/master/examples/http-requests)
that uses [tokio-curl](https://github.com/tokio-rs/tokio-curl)
to make asynchronous HTTP requests as part of a
a Cap'n Proto method call.
Another exciting possibility would
be to use
[futures-cpupool](https://crates.io/crates/futures-cpupool)
to farm out compute-bound or disk-IO-bound work to a pool of worker threads.





