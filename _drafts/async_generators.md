---
layout: post
title: async generators
author: dwrensha
---

Until recently,
the concept of *generators*, or *resumable functions*,
seemed to me like a cute idea
with only niche use cases.
Sure, I had heard that generators
in [python](https://www.python.org/dev/peps/pep-0255/)
and
[javascript](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Iterators_and_Generators#Generators)
could make certain things much nicer,
but how often does one really need to have
what appears be nothing more than a fancy iterator?
It wasn't until I followed this
this [Rust RFC thread](https://github.com/rust-lang/rfcs/issues/1081)
that the potential of generators in Rust started to dawn on me.
Today they are my number-one most-desired new feature for the language, because I
believe they are the missing piece in Rust's async story.


{% highlight rust %}
pub enum Yield<T, R> {
    Value(T),
    Return(R)
}

pub trait Generator {
    type Value;
    type Return;
    fn next(&mut self) -> Yield<Self::Value, Self::Return>;
}
{% endhighlight %}

{% highlight rust %}
fn fib_with_sum(n: usize) -<u64>-> u64 {
     let mut a = 0;
     let mut b = 1;
     let sum = 0;
     for _ in 0 .. n {
         yield a; // <--- new keyword "yield"
         sum += a;
         let next = a + b;
         a = b;
         b = next;
     }
     return sum;
}
{% endhighlight %}

Here I've used some strawman syntax `fn foo() -<T>-> R`, which denotes that `foo`
is a generator function that produces some indeterminate number of `T` values
and then finishes by producing an `R`. Each `yield` corresponds to a `T`, and the final
`return` corresponds to the `R`.
When you call `fib_with_sum()`, the value you get back is a generator,
which can be used by calling `next()`:
{% highlight rust %}
let mut g = fib_with_sum(5);
g.next(); // Yield::Value(0)
g.next(); // Yield::Value(1)
g.next(); // Yield::Value(1)
g.next(); // Yield::Value(2)
g.next(); // Yield::Value(3)
g.next(); // Yield::Return(7)
{% endhighlight %}



{% highlight rust %}

use std::io::{Error, ErrorKind, Result};

type AsyncStatus = (Token, Interest);

pub trait AsyncWrite {
    fn write(&mut self, bytes: &[u8]) -<AsyncStatus>-> Result<()>;
}

pub trait AsyncRead {
    fn try_read(&mut self,
            buf: &mut [u8],
            min_bytes: usize)
        -<AsyncStatus>-> Result<usize>;

    fn read(&mut self,
            buf: &mut [u8],
            min_bytes: usize)
        -<AsyncStatus>-> Result<usize>;
    {
       let n = try!(yield from self.try_read(buf, min_bytes);
       if n < min_bytes {
           Err(Error::new(ErrorKind::UnexpectedEof, ""))
       } else {
           Ok(n)
       }
    }
}

{% endhighlight %}
