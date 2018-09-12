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
   | N<{T}, {N=T}>
   
N = C // "classes", e.g., `Vec`
  | S // "structs", e.g., `u32` or `Option`
```

As in Rust, there is a basic "well-formedness" (WF) predicate that
requires that e.g. `share(a) share(b) T` is only possible if `b: a`.

## Structs vs classes

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

## Positional vs associated types

You'll note that structs/classes have two sorts of arguments:

```
Vec<u32>
Mutex<Data = u32>
```

Positional arguments are expected to be more common, but they play a
very specific role: they always refer to types that are **owned** by
the containing type, and in particular owned outside of any "shared
mutability" cell. This also allows us to do ergonomic transformations
on them; it also means they can be covariant.

Associated arguments (`Data = u32`) are more flexible. They can play
many roles; however, as a result, we are more limited in what we can
do there. Such types are invariant.

## Representation of shares and borrows

### share and own are represented the same way at runtime

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

### optimizing shared representation

Whenever a `share Foo` is stored directly on the stack, we can likely
convert it into a pointer representation. This is not intended to be
visible to the end user. We should work out the rules for such
optimizations.

### borrow is not

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

## Mutability

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

## Normalization of sharing

`share T` types are "normalized" according to the following rules. If
T1 normalizes to the type T2, that means that T1 and T2 are considered
to be **the same type** by the system.

- `share(r1) share(r2) T` becomes `share(r2) T`
  - Note that "well formedness" requires that `r2: r1`
- `share(r1) S<{T}, {N=U}>` becomes `S<{share(r1) T}, {N=U}>`
  - Structs are eagerly cloned
  - Simple case: `share(r1) u32 = u32`
  - Medium case: `share(r1) Option<u32> = Option<u32>`
  - Complex case: `share(r1) Option<own Vec<u32>> = Option<share Vec<u32>>`
  - Associated case: `share(r1) Foo<T = Bar> = Foo<T = Bar>` (`Bar` is unchanged)
- `share(r1) C<{T}, {N=U}>` becomes `share(r1) C<{share(r1) T}, {U=T}>`
  - Classes retain the "shared" modifier (but still propagate it inward)
  - Simple case: `share(r1) String` is fully normalized
  - Medium case: `share Vec<own u32>` is fully normalized
    - Alternative: it becomes `share Vec<share u32>` which then becomes `share Vec<own u32>` again =)
  - Complex case: `share Vec<own Vec<u32>>` becomes `share Vec<share Vec<u32>>`
  - `share Mutex<Data = own Vec<u32>>` becomes `share Mutex<Data = own Vec<u32>>`

### Interaction with contra-variance and cells

The normalization rules above work because we require that `Foo<Bar>`
implies that the `Foo` owns a copy of `Bar` and that -- if `Foo` is
shared -- that `Bar` is also to be considered shared. This means that
the "fundamental capabilities" offered by `share Foo<own Bar>` are
equivalent to those offered by `share Foo<share Bar>` (also, the
memory layout is the same).

These things are not true when you have contra-variance or cells. We
enforce this rule by forbidding positional type parameters from
appearing in a contra-variant or in-variant position.

Consider this example:

```
class FnPtr {
  type Arg;
  
  f: fn(own Arg)
}
```

If we had a value of type T where T is

```
own FnPtr<Arg = own Vec<u32>>
```

and we shared it to `share T`, it would be wrong to normalize `share T` to

```
share FnPtr<Arg = share Vec<u32>>
```

In particular, that would allow us to call the function `f` with a
`share Vec<u32>` even though it expects an `own Vec<u32>`. Seems bad.

#### Cells and shared mutability 

Note that the fundamental cell type (`UnsafeCell`, in Rust) is modeled
using an associated type, forcing it too to be invariant:

```
class UnsafeCell {
  type Data;
  
  data: Self::Data
}
```

This is because `UnsafeCell` affords a capability -- the ability to
mutate `self.data` even when shared -- that ordinary types do not
have.

#### Reflection on semver, shared mutation, and Rust

This design makes invariance and shared mutability very present in the
"external interface" offered by a class. It means that one cannot
causally add a `Mutex<T>` to a type definition if `T` is a generic
defined on the class: that would require you to convert `T` from a
positional type into an associated type.

Rust does not require the same gyrations. However, this does not imply
that adding a `Mutex<T>` in Rust is not a "breaking change" (in the
sense that your clients may stop compiling): it still makes `T`
invariant, which can lead to subtle and surprising lifetime
errors. (It also affects auto traits, but that is orthogonal.)

The theory is that it makes sense to present shared mutability very
differently from ordinary ownership, both because it is unusual but
also because it affords more capabilities than an ordinary type. In
exchange for doing that, this means we can make ordinary ownership
more ergonomic, by propagating sharing inward
automatically. (Moreover, using a distinct syntactic form means that
people can tell whether sharing will propagate without even looking at
the struct definition.)

Another way to look at it is that Rust forces all types to be
conservative in order to account for the possibility that they *may*
add shared mutation in the future, when in fact that is a very rare
occurrence.

## Subtyping rules

The full subtyping rules are as follows:

```
share(r1) T1 <: share(r2) T2 :-
  r1: r2,
  T1 <: T2.

borrow(r1) T1 <: borrow(r2) T2 :-
  r1: r2,
  T1 == T2. // sloppy, really this can only be a C

N<{T1}, {N=U1}> <: N<{T2}, {N=U2}> :-
  {T1 <: T2},
  {U1 == U2}.
```

Here `==` means "equal types" can implements the normalization rules
given earlier.
