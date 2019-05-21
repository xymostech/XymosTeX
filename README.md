# XymosTeX [![Build Status](https://travis-ci.com/xymostech/XymosTeX.svg?branch=master)](https://travis-ci.com/xymostech/XymosTeX) [![Coverage Status](https://coveralls.io/repos/github/xymostech/XymosTeX/badge.svg?branch=master)](https://coveralls.io/github/xymostech/XymosTeX?branch=master)

This project is an attempt to build an implementation of TeX in Rust. The overall goals of this project are:

1. Be a complete TeX implementation (see [more info about the goals](goals.md) for a detailed discussion of what this means)
2. Personally, to learn more intrinsically how TeX works by reading the TeXbook and trying things instead of by just reading the TeX source
3. Allow for better debugging of TeX (especially w.r.t. macro expansion) to provide helpful information for other TeX-like projects like [KaTeX](https://github.com/KaTeX/KaTeX).

## Status

Currently, XymosTeX has reached the point where is it Turing complete (that is, it implements things like macro expansion, conditionals, assignments) and can interpret very basic horizontal and vertical boxes

The next body of work will be creating DVI output from the parsed vertical and horizontal boxes.

## Trying it

Because XymosTeX is under development, the best way to try it is to clone the repo and build it from in there.

```
$ git clone https://github.com/xymostech/XymosTeX.git
$ cd XymosTeX
$ cargo run
\def\hello #1{Hello, #1!}
\hello{World}
 Hello World!
```

The most fun and impressive thing that XymosTeX can do is calculate primes for you:

```
$ cargo run --release < examples/primes.tex
2, 3, 5, 7, 11, 13, 17, 19, 23, and 29
```

## Contributing

I'm not currently taking contributions for small changes towards the current goals, but if you have a large area of TeX that you're interested in trying to tackle then I'd love to hear from you! Some examples of "large areas":

* Line breaking/page breaking
* Hyphenation
* Math layout
* Error recovery
* DVI output

## License

XymosTeX is licensed under the [MIT License](LICENSE)
