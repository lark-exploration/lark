# Lark

Lark is an experimental programming system which includes a programming language, a compiler, a build system, IDE support, and a VSCode plugin. As Lark is both experimental and very much in its early stages, you’ll find varying levels of completeness.

The primary goal of Lark is to experiment with new approaches to designing a programming language using Rust as an implementation language. By focusing on a breadth-first, holistic approach to the implementation, we hope to find useful implementation techniques that could be more broadly useful, especially as implementation techniques in Rust itself. We are also exploring approaches to language features that could be more broadly useful, but those explorations are more speculative at this time.

Lark draws heavy inspiration from other languages, notably the Rust programming language. Similar to Rust, it has a system of permissions drawn from Rust’s borrow checker to help ensure safe, efficient code without the need of a garbage collector. While Lark doesn’t always share the exact design of Rust, it intends to start from many of the same design ownership axioms:

* the language’s dominant usage patterns do not require garbage collection
* the language is memory-safe by design
* the language is thread-friendly, guaranteeing freedom from data-races and strongly encouraging patterns that eliminate other kinds of race conditions

Lark, both as a tool and as a language, was designed to treat both the base-line compiler and interactive code editing as first class concerns.

To accomplish this, the Lark system relies heavily on [Salsa](https://github.com/salsa-rs/salsa), a framework for doing efficient, incremental computation. This allows the compiler to request a piece of information, like the types of a function’s parameters, and automatically compute any derived information it needs. This makes the compiler incremental-by-design. By describing the necessary computations in terms of Salsa, the compiler and language server automatically do just the computation they need in order to service demands for information.

Salsa is already being used in [experimental third-party Rust analysis tool](https://github.com/rust-analyzer/rust-analyzer), and with Lark we hope to further explore the capabilities that Salsa makes possible.

# What does it look like?

Lark is still very experimental, with many core features subject to change. That said, the current version looks like this:

```rust
struct Adder {
    x: uint
    y: uint

    sum() -> uint {
        self.x + self.y
    }
}

def main() {
    let adder = Adder(x: 4, y: 7)

    debug(adder.sum())
}
```

# Goals

The current goals of Lark focus on building a new compiler on top of the Salsa incremental computation system. We’re also exploring a variety of functionality that leverages this, including IDE support, compiler, and a REPL/interpreter.

The Lark language is intended to gain functionality that considers both expressiveness and efficiency, with an emphasis on the readability and maintainability cost of language features. As such, Lark aims to take an 80/20 rule to design: if a design can help programmers 80% of the time, even though 20% of the time it might make something slightly more cumbersome, then this is worthwhile. 

Additionally, Lark is being designed both as a language and as a system to take IDE support as a first-class feature.

# Non-Goals

Lark, especially in its pre-alpha state, is not intended to be used for any commercial products. It is instead intended to be used as a research language. 

Lark isn’t intended to research all areas of programming language design, or to accrue language features broadly. It instead focuses on the set that can be used together well and to show they can be built with incremental compilation in mind. The initial designers of Lark are also very involved in and committed to the Rust language, so it is also focused on language features that are spiritually similar to the design goals of Rust, in hopes of exploring language design problems that can inspire future designs in Rust.

Lark, in its current state, does not attempt to create the most efficient code. Instead, it defers optimization to external compilers. That said, the Lark language design does consider efficiency, and tries to avoid designs that in practice would produce significant and measurable reductions in whole-program efficiency compared to Rust.

# Current Status

Lark is in a pre-alpha stage. It’s built to explore many areas of interest, but it is not a complete exploration of any one area. The following is the status of each sub-area:

* Language features
  * If conditional
  * Functions and function calls
  * Structs, struct instance creation, methods, and member access
  * Strings
  * `bool` and `uint`
  * Other
* IDE support
  * Language Server Protocol (LSP) support
    * Find-all-references (mostly working)
    * Goto-definition (mostly working)
    * Refactor/rename (based on find-all-references above)
    * Errors-as-you-type
    * Type-on-hover
    * (note: at this time, completion is not yet implemented)
  * VSCode plugin
    * All LSP functionality above is supported in the VSCode plugin
* Type-checker
  * Basic type checking across supported language features
* Permissions-checker
  * Ownership inference
  * Initialization checking
* Interpreter
  * Run Lark code via interpreter
  * Interactive REPL
* Code generation
  * Output to Rust
  * Planned:
    * Output to C
    * Output to WASM (possibly)
* Error reporting
  * Rust-like error pretty printer
  * Minimal error messages
* Internal tests
  * Compiler tests
  * IDE-based tests
* Internal design
  * Salsa-based incremental compilation
  * Multi-threaded compilation
  * Multi-threaded IDE support

## How mature is Lark?

Lark is still very much in the unstable/pre-alpha stage. It is missing basic functionality that is common even in the simplest production systems. Syntax may change significantly. Core concepts may be added, removed, or changed as we learn from the experiments.

It is not recommended that anyone use Lark in any commercial setting at this time.

## Why Lark? (aka Why not do this as part of another language?)

Lark originally grew from multiple pet projects that Jonathan and Yehuda had created separately. Once the two noticed the strong overlap in design goals, the projects were merged and work quickly grew to include the incremental computation system, Salsa, that Niko had begun work on.

The original motivations came from seeing multiple areas in existing languages where alternate designs or implementation choices could have been made. We wished that those designs were explored, even though we weren’t sure that those explorations would succeed.

Lark’s design philosophy is derived closely from Rust’s, so we assume that if Lark proves out parts of the feature set, it will make sense to experiment with a similar feature set in Rust. That said, Lark’s design decisions sometimes diverge strongly from Rust’s, so successfully proving out a feature in Lark may not always mean that a straight-forward port to Rust is possible.

Designing from scratch also allows for both easier experimentation and a focus on a few core principles.

## How does this relate to Rust?

Though Lark was created by engineers who work on Rust, it is a separate project. Lark is not an official Rust project, but rather a research project done to explore what’s possible in modern languages and tools.

One possible way to view Lark is similar to the relationship between Servo and Firefox. Servo started as an independent research project that would later also feed ideas and code to Firefox.

It is hoped that many of the techniques explored in Lark can become techniques that `rustc` adopts and that become RFCs in the Rust language.

## When will Lark have feature X?

Declaration forms, conditionals, and loops are all macros in Lark. It is the general hope that the macro system should unlock the ability to express a wide range of capabilities without having the necessarily grow Lark’s core language features. (We realize that this is a controversial perspective, and strongly diverges from Rust’s macro model.)

In particular, Lark uses a similar HIR setup as Rust, which allows a small number of core language primitives to serve as the target for a much larger syntax surface area. Unlike Rust, Lark implements virtually all syntax as macros, which allows external experimentation with syntactic forms, as well as “edition”-style evolution for nearly all syntax.

## How can I help?

If you’re interested in helping out, there are a lot of areas you can help.  If you’re interested in working on the compiler, you can look at the outstanding issues. You can read more about the [compiler internals](docs/internals.md).

We will also be putting in effort to design areas that we know need to be designed next. Rather than proposing a language feature, we encourage people to read the roadmap and look at the active design discussions for the parts of the language that are being actively created. 

There are also plenty of coding opportunities outside of working on the compiler itself, including fleshing out the test suite with additional tests, writing samples and filing issues for areas that are expected to work, and trying out the IDE support.

We’ll also try to make “quest” issues available when possible. These will have more detailed instructions with how to contribute.

# License

Lark is dual-licensed under Apache 2.0 and MIT. 

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
