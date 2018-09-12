# Types in Lark

NB. This discussion here describes a **formal type system** for Lark,
not the **surface syntax** exposed to users. In particular, in this
system, a type like `Foo` means an "owned Foo", but in Lark it is
expected to write `own Foo`. In a formal setting, having own be the
default makes sense, since ownership is (in a sense) the "weakest"
type: i.e., a shared owned thing is shared, not owned.

```
T  = share(r) T
   | borrow(r) C<{T}>
   | C<{T}> // "classes", e.g. Vec<T> or String
   | S<{T}> // "structs", including things like u32
```

As in Rust, there is a basic "well-formedness" (WF) predicate that
requires that e.g. `share(a) share(b) T` is only possible if `b: a`.

# Structs vs classes

I am assuming that there is a distinction of a struct vs a class.  A
struct is (sort of) a "value type". Structs are special because a
shared struct is just copied -- so e.g. the type `share u32` just
normalizes to `u32`.

To make these semantics work, structs cannot **directly** embed a
class within. That is, you cannot have something like

```
class Foo {
}

struct Bar {
  foo: Foo
}
```

## Generic structs embedding classes

However, although structs cannot directly embed classes within
themselves, they *can* be generic, and those generics can be
instantiated to a class. So for example `Option<Vec<u32>>` is a valid
type. These "struct-class hybrids" behave a bit differently from a pure struct:

- They are linear. So when you give ownership away, you lose the
  original (unless you explicitly clone).
- When shared, the sharing propagates into the generic. So `shared
  Option<Vec<u32>>` normalizes to `Option<shared Vec<u32>>`.

**Why forbid embedding?** The reason that we forbid classes from being
directly embedded in structs is exactly that we cannot propagate the
"shared" notion into the interior of the struct if there is no
generic.

# Representation of shares and borrows

## share and own are represented the same way at runtime

`own T` and `share T` share a representation: the difference is just
in what destructors the compiler will run, essentially, and what
permissions you have. This means though that if you have a struct
`Foo` with 3 fields and you take a `share Foo`, you get 3 fields
copied over to you.

This has a few notable advantages:

- It means that pointers don't matter when you propagate sharing (see
  below). This implies we can have an `indirect` feature that allows
  you to add boxing at any point without it having any effect on the
  rules below.
- It means that subslicing for `String` does not require allocation:
  you can take a `share String` and return a new `share String`
  without changing your representation at all.
  - In Rust, you cannot return a `&String` unless you have something
    to point at; this is why we have to switch to `&str`, so we can
    carry the length outside of the pointer.

The first one is the clincher for me: the latter could theoretically
be overcome by arenas, though at unknown runtime cost.

## borrow is not

The arguments above cannot apply to borrow, unless we want to require
that all primitive types have the same size as a pointer (hint: I
strongly suspect we don't). In fact, I am not at all convinced that
`borrow` is even a mode in the normal sense, and I think there may be
value in not thinking of it that way.

Example: Consider this struct, which -- when owned -- is represented
as a single byte.

```
struct Foo {
  x: u8
}  
```

How can you represent `borrow Foo`? There has to be a pointer to a
byte, which will not be 1 byte in size.

# Mutability

The general rule in Lark is that local variables cannot be
'reassigned' but fields can. That is, one cannot do `x = 3` but one
*can* do `x.y = 5`. I would consider going further and say that fields
which can be reassigned must be declared as `mut`.

The advantage of this is that if you have:

```
struct Foo<T> {
  values: Vec<T>,
  mut count: usize,
}
```

and you have a `foo: borrow Foo<T>`, then you know that `foo.values` is immutable.
This implies two things:

- the type `borrow Foo<T>` can be covariant with respect to `T`
- you can permit a shared borrow of `values` to overlap a `borrow` of `Foo<T>`

So e.g. this would be fine:

```
impl Foo<T> {
  def borrow other_thing() {
    for x in self.values { // shares `self.values` during loop...
      if something(x) {
        self.increment_count(); // but that doesn't conflict
      }
    }
  }

  def borrow increment_count() {
    self.count += 1;
  }
}
```

Not a complete solution to the "which fields do you modify" problem,
but should reduce its incidence.

# Variant 1: Strong normalization

This "strong normalization" variant is my preferred variant, but it
requires us to make "shared mutable" more *visible* to end
users. We'll come to that.

The basic idea is that a `share T` can be normalized according to the following
rules:

- `share(r1) share(r2) T => share(r2) T` (WF requires then that `r2: r1`)
- `share(r1) C<{T}> => share(r1) C<{share(r1) T}>`
- `share(r1) S<{T}> => S<{share(r1) T}>`

Some examples:

- `share(r1) Vec<own Foo> => share(r1) Vec<share(r1) Foo>`
- `share(r1) Option<own Foo> => Option<share(r1) Foo>`
  - because `Option` is a "struct" (value type)
- `share(r1) borrow(r2) Foo` -- no change here.  

## Interaction with shared mutability

"shared" propagation doesn't work if you have things like `Mutex<T>`
that permit mutation even when shared. To accomodate that, I think we should
declare type parameters that appear in a shared mutabile context somewhat
different. Let's adopt for now the `cell` term from Rust. In that case,
we might have:

```
class Foo<cell T> {
  mutex: MutexCell<T>
}
```

Alternatively, a "cell" -- something that has shared mutation interior to it --
might another kind of class.

Alternatively, we might just not do downward propagation through
classes, but that seems unfortunate, since now there is a difference
between `shared Vec<shared S>` and `shared Vec<own S>`.

# Variant 2: Weak normalization

One could adopt a "weak normalization" variant in which we only
propagate sharing through structs, not classes:

- `share(r1) share(r2) T => share(r2) T` (WF requires then that `r2: r1`)
- `share(r1) S<{T}> => S<{share(r1) T}>`

In this model, then, shared mutability can be incorporated freely.
However, the downside is that `shared Vec<String>` and `shared
Vec<shared String>` are distinct types, despite the fact that there is
no operation you can do with the former that you cannot do with the
latter.

# Reflects on variant 1 vs variant 2

The example of `shared Vec<String>` normalizing to `shared Vec<shared
String>` gets at the heart of the issue: because we know that `Vec`
will never have shared mutation, there is no reason **not** to do this
normalization. But in Variant 2 we still cannot do it, because we have
adopted a more conservative rule that allows any type to add shared
mutation without it affecting the "public interface".

In practice, though, the "semver compatibility" *is* still affected,
so this is something of a lie. In Rust, shared mutation doesn't affect
any of the "visible" public interface, but it is typically a breaking
change anyway: it affects variance, for example, and -- unless one
uses `Mutex` -- can affect thread safety as well. This breaking
change, however, is "silent" in that it is not marked in the struct
syntax. The main reason this has not become a problem is that shared
mutation is so rare.

In principle, we can infer the `cell` annotation, but we also want to
think about what to do with traits. e.g., if you have

```rust
trait Foo<T> { ... }
```

then can we transform a `shared Foo<T>` to a `shared Foo<shared T>`?
I would like the answer to be yes, but then we have to move the `cell`
declaration into the trait, and it will in turn restrict the types
that implement the trait.

# Connection between cell and variance

Hmm, there is obviously a connection between cell and variance. The
"cell" is basically a kind of invariance annotation. Maybe, indeed, it
is useful to think of it as a variance annotation -- and perhaps
(then) not to use `cell` but some other term. Let's think a bit about
variance.

(Also a note: We are making a pretty deep assertion here that is using
the **fundamental capabilities provided by the underlying type** to
say that all methods must be legal to transform in this way. For
example, consider something like `contains`.)

# Variance

We may or may not want a strict notion of *subtyping* but we are going
to want at minimum "recursive coercions" that permit `share(r1) T` to
be "upcast" to `share(r2) T` where `r1: r2`. This would be useful for
example in code like this:

```
def foo(x: share(a) Foo, y: share(b) Foo) {
  let v: Vec<share(c) Foo> = [x, y]; // where a: c, b: c
}
```

Similarly, we probably want this to propagate deeply, so that e.g. one could also do:

```
def foo(x: Vec<share(a) Foo>, y: Vec<share(b) Foo>) {
  let v: Vec<Vec<share(c) Foo>> = [x, y]; // where a: c, b: c
}
```

For this to work, we must have a notion of variance.

I touched briefly on variance in the previous section. I'm going to
assume for now that only *some* fields are declared as `mut`. I think
further that any type parameters which appear in the type of a `mut`
field should also be declared `mut` by the user (that could be
inferred, as Rust does; but note that it is transitive).

Ideally, I think, we would have "multiple" notions of variance in
Rust, depending on the mode.
