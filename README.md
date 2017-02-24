# Cap'n Proto for Rust

[![Build Status](https://travis-ci.org/llogiq/capnproto-rust.svg?branch=master)](https://travis-ci.org/llogiq/capnproto-rust)
[![Windows build status](https://ci.appveyor.com/api/projects/status/github/llogiq/capnproto-rust?svg=true)](https://ci.appveyor.com/project/llogiq/capnproto-rust)
[![crates.io](http://meritbadge.herokuapp.com/capnp)](https://crates.io/crates/capnp)

[documentation](https://docs.capnproto-rust.org/capnp/)

[blog](https://dwrensha.github.io/capnproto-rust)

## introduction

[Cap'n Proto](https://capnproto.org) is a type system for distributed systems.

With Cap'n Proto, you describe your data and interfaces
in a [schema file](https://capnproto.org/language.html), like this:

```capnp
@0x986b3393db1396c9;

struct Point {
    x @0 :Float32;
    y @1 :Float32;
}

interface PointTracker {
    addPoint @0 (p :Point) -> (totalPoints :UInt64);
}
```

You can then use the [capnp tool](https://capnproto.org/capnp-tool.html#compiling-schemas)
to generate code in a [variety of programming languages](https://capnproto.org/otherlang.html).
The generated code lets you produce and consume values of the
types you've defined in your schema.

In Rust, the generated code for the example above includes
a `point::Reader<'a>` struct with `get_x()` and `get_y()` methods,
and a `point::Builder<'a>` struct with `set_x()` and `set_y()` methods.
The lifetime parameter `'a` in these generated struct types
is a formal reminder that they contain borrowed references to
the raw buffers of bytes that make up the underlying Cap'n Proto messages.
Those underlying buffers are never actually parsed into
separate data structures -- Cap'n Proto's wire format also serves as its in-memory format.
That is, there is no encode or decode step!
It's in this sense that Cap'n Proto is
["infinity times faster"](https://capnproto.org/news/2013-04-01-announcing-capn-proto.html)
than alternatives like Protocol Buffers.

The generated code also includes
a `point_tracker::Server` trait with an `add_point()` method,
and a `point_tracker::Client` struct with an `add_point_request()` method.
The former can be implemented to create a network-accessible object,
and the latter can be used to invoke a possibly-remote instance of a `PointTracker`.

## related repositories
- [capnproto-rust](https://github.com/dwrensha/capnproto-rust) (the repo you're looking at right now):
  Runtime library for dealing with Cap'n Proto messages.
- [capnpc-rust](https://github.com/dwrensha/capnpc-rust): The Rust code generator plugin, including
  support for hooking into a `build.rs` file in a `cargo` build.
- [capnp-futures](https://github.com/dwrensha/capnp-futures-rs): Support for asynchronous reading and writing
  of Cap'n Proto messages.
- [capnp-rpc-rust](https://github.com/dwrensha/capnp-rpc-rust): Object-capability remote procedure call
  system.

## examples

[addressbook serialization](https://github.com/dwrensha/capnpc-rust/tree/master/example/addressbook),
[RPC](https://github.com/dwrensha/capnp-rpc-rust/tree/master/examples)

## who's using capnproto-rust?

- Sandstorm's [raw API example app](https://github.com/dwrensha/sandstorm-rawapi-example-rust) and
  [collections app](https://github.com/sandstorm-io/collections-app)
- [leaf](https://github.com/autumnai/leaf)
- [fractalide](https://github.com/fractalide/fractalide)
- [combustion-engine](https://github.com/combustion-engine/combustion/tree/master/combustion_protocols)

## unimplemented / future work

- [Orphans](https://capnproto.org/cxx.html#orphans)
- [Dynamic reflection](https://capnproto.org/cxx.html#dynamic-reflection)
- Pointer constants and defaults


