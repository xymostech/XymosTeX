# Goals of XymosTeX

The overall goal of XymosTeX is to be a complete implementation of TeX. What does that mean? [According to Knuth](http://texdoc.net/texmf-dist/doc/generic/knuth/tex/tripman.pdf), a requirement is that it correctly translates trip.tex, a "torture test" for TeX. So, I will use that as our criterion for being correct and complete.

The other goal is orthogonal to the actual completeness or correctness of the implementation is the main reason that I'm going to the trouble of reimplementing TeX at all. Instead of trying to add additional functionality to the actual rendering side of TeX, my goal is to add a more reasonable debugging/tracing facility for TeX. The original reasoning for attempting the wild goal of reimplementing TeX was as an aide for implementing features in [KaTeX](https://github.com/KaTeX/KaTeX), the web LaTeX math renderer. With plain TeX, it is very difficult to decipher what many of the more complicated LaTeX math macros expand to (`\begin{align}` and friends being the most useful). With a reasonable debugging/tracing interface, it would be much easier to understand how these complicated macros work.

An auxiliary hope is that I will understand how core TeX works much better. To that end, I am going to try to make a complete implementation of TeX without looking at the original source of TeX and will only consult the TeXbook and look at the output of TeX. I'm worried that letting myself look at the source will encourage me to simply copy what is there instead of deeply understanding what is happening at its core. Maybe that is foolish, but it is interesting.

My plan is to implement features in stages, with each stage having a specific goal.

## Stage 1: Calculating Prime Numbers

**Status**: Done!
**Difficulty**: Easy
**Condition for success**: correctly interpreting [a series of macros, assignments, and conditionals that produce an output of prime numbers](examples/primes.tex) (this is a simplified version of the same function found in the TeXbook)

The goal of this first stage is to get some of the core parsing and lexing working. A large part of this stage will be ensuring that assignment, expansion, and conditionals work correctly.

Understanding and implementing the concepts in this stage is actually fairly difficult, but I have already gotten this working in [my incomplete JavaScript implementation of TeX](https://github.com/xymostech/js-tex-parser), so I can simply use a similar implementation here. Most of the problems here will be around translating JavaScript concepts into Rust ones.

## Stage 2: Making Boxes

**Status**: Done!
**Difficulty**: Medium
**Condition for success**: correctly evaluating and printing metrics for boxes and building horizontal boxes from [commands that build horizontal boxes at different widths with glue](examples/boxes.tex)

Instead of producing simple textual output as a result of the parsing, in this stage I will begin producing TeX boxes. I'll need to begin parsing character metrics for the individual characters to get the sizes for individual characters, and start allowing glue inside of horizontal boxes. I'll need to add box registers, and I'll need to allow setting the glue in a box, allow for reading metrics about the boxes, and allow for nesting boxes inside of other boxes.

## Stage 3: Vertical mode

**Status**: Done!
**Difficulty**: Medium
**Condition for success**: correctly parsing, building, and measuring the vertical boxes from [commands that build vertical boxes and enter and leave vertical and horizontal mode using different techniques](examples/vertical.tex)

At this point, I'll be able to start parsing from (internal) vertical mode, have that correctly call out to a (restricted) horizontal mode, and then return back to vertical mode to produce vertical boxes. This will add vertical glue as well.

## Stage 4: DVI Output

**Status**: Done!
**Difficulty**: Medium
**Condition for success**: generate a DVI from a [series of commands creating vertical and horizontal boxes with spacing and characters in them](examples/dvitest.tex) that is content-identical to [the DVI produced by TeX run on the same commands](examples/dvitest.dvi)

For this stage, I shouldn't have to work on the parser, because the current parser should be able to interpret the example file into boxes already. Instead, I'll be taking those generated boxes and turning them into a proper DVI file that represents the contents of the boxes.

Because the DVI file format isn't rigorous about how certain commands are used (like the orders of certain commands, several comments, which variables are used) and also because there are many different ways to produce the same output from a given box, I'll have to figure out a good way to ensure that the DVI that I produce is the same as the one that TeX produces, because I won't be able to simple byte-compare the DVI files.

## Stage 5: Math Parsing

**Status**: Not yet started
**Difficulty**: Medium
**Condition for success**: Be able to parse a complicated mathematical expression into a box

## Stage 6: Paragraph & Line Breaking

**Status**: Not yet started
**Difficulty**: Hard
**Condition for success**: ???

## Stage 7: ???

**Status**: Not yet started
**Difficulty**: ??? (Probably hard)
**Condition for success**: ???

## Stage 8: trip.tex

**Status**: Not yet started
**Difficulty**: Hard
**Condition for success**: Correctly interpreting trip.tex according to [the manual](http://texdoc.net/texmf-dist/doc/generic/knuth/tex/tripman.pdf).

Unaccounted for:
* Error recovery
* Alignment
* Headings & Footers
* \edef, \outer\def, \long\def
* \input
* \csname, \string
* Hyphenation
