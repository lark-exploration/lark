/// Use to declare a "newtype'd" index that can be used with `IndexVec`.
/// The simplest usage is just:
///
/// ```
/// index_type! {
///     $v struct Foo { .. }
/// }
/// ```
///
/// where `$v` is whatever visibility you want (`pub`, `crate`, etc).
///
/// Instances offer several methods and also implement a number of convenient
/// traits:
///
/// - `Foo::new(22)` -- make a new `Foo` from a `usize` (works in constants, too)
/// - `Foo::from_u32(22)` -- make a `Foo` from a `u32` (works in constants, too)
/// - `foo.as_u32()` -- extract the inner value as a `u32`
/// - `foo.as_usize()` -- extract the inner value as a `usize`
/// - `Foo: From<usize>` -- you can also use the `From` trait to construct from a usize
/// - `Foo: From<u32>` -- ...or a u32
///
/// Index types also implement the usual suspects (Copy, Clone, Debug, PartialOrd,
/// Ord, PartialEq, Eq, Hash) so they can be used with maps and things.
///
/// ### Storage
///
/// Internally, index types use a `NonZeroU32` for storage. This means that they
/// cannot exceed 2^32 - 1 in value (the code will assert this at runtime). It also means
/// that `Option<Foo>` is just one `u32` in size.
///
/// ### Configuring the newtype
///
/// Before the `..` you can also add some configuration options. Example:
///
/// ```
/// newtype_index! {
///     struct Foo {
///         debug_name[Bar],
///         ..
///     }
/// }
/// ```
///
/// The options are:
///
/// - `debug_name[$name]` -- change how the `Debug` impl prints out. If you put
///   just `debug_name[]`, then we will not emit any `Debug` impl at all.
#[macro_use]
macro_rules! index_type {
    ($(#[$attr:meta])* $visibility:vis struct $name:ident { $($tokens:tt)* }) => {
        index_type! {
            @with {
                attrs[$($attr)*],
                visibility[$visibility],
                name[$name],
                max[std::u32::MAX - 1],
                debug_name[stringify!($name)],
            }
            @tokens {
                $($tokens)*
            }
        }
    };

    // Base case: no more options
    (
        @with {
            attrs[$($attrs:meta)*],
            visibility[$visibility:vis],
            name[$name:ident],
            max[$max:expr],
            debug_name[$($debug_name:expr)?],
        }
        @tokens {..}
    ) => {
        index_type! {
            @with {
                attrs[$($attrs)*],
                visibility[$visibility],
                name[$name],
                max[$max],
                debug_name[$($debug_name)?],
            }
        }
    };

    // Consume a `debug_name[]` option
    (
        @with {
            attrs[$($attrs:meta)*],
            visibility[$visibility:vis],
            name[$name:ident],
            max[$max:expr],
            debug_name[$debug_name1:expr],
        }
        @tokens {
            debug_name[$($debug_name2:ident)?],
            $($tokens:tt)*
        }
    ) => {
        index_type! {
            @with {
                attrs[$($attrs)*],
                visibility[$visibility],
                name[$name],
                max[$max],
                debug_name[$(stringify!($debug_name2))?],
            }
            @tokens {
                $($tokens)*
            }
        }
    };

    // Generate the type definition and various impls
    (@with {
        attrs[$($attrs:meta)*],
        visibility[$visibility:vis],
        name[$name:ident],
        max[$max:expr],
        debug_name[$($debug_name:expr)?],
    }) => {
        #[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
        $(#[$attrs])*
        $visibility struct $name {
            private: std::num::NonZeroU32
        }

        impl $name {
            $visibility const fn new(index: usize) -> Self {
                // This is a wacky assert that is compatible with a
                // const fn.  It will evaluate to an out-of-bounds
                // access if `index >= $max`.
                let v: u32 = [index as u32][(index >= ($max as usize)) as usize];
                unsafe { Self { private: std::num::NonZeroU32::new_unchecked(v + 1) } }
            }

            $visibility const fn from_u32(index: u32) -> Self {
                // This is a wacky assert that is compatible with a
                // const fn.  It will evaluate to an out-of-bounds
                // access if `index >= $max`.
                let v: u32 = [index][(index >= $max) as usize];
                unsafe { Self { private: std::num::NonZeroU32::new_unchecked(v + 1) } }
            }

            $visibility fn as_u32(self) -> u32 {
                self.private.get() - 1
            }

            $visibility fn as_usize(self) -> usize {
                self.as_u32() as usize
            }
        }

        impl From<usize> for $name {
            fn from(v: usize) -> $name {
                $name::new(v)
            }
        }

        impl From<u32> for $name {
            fn from(v: u32) -> $name {
                $name::from_u32(v)
            }
        }

        impl indexed_vec::Idx for $name {
            fn new(v: usize) -> $name {
                $name::from(v)
            }

            fn index(self) -> usize {
                self.as_usize()
            }
        }

        index_type! {
            @debug_impl {
                name[$name],
                debug_name[$($debug_name)?]
            }
        }
    };

    // User requested no debug impl
    (
        @debug_impl {
            name[$name:ident],
            debug_name[]
        }
    ) => {
    };

    // Generate `Debug` impl as user requested
    (
        @debug_impl {
            name[$name:ident],
            debug_name[$debug_name:expr]
        }
    ) => {
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple($debug_name)
                    .field(&self.as_usize())
                    .finish()
            }
        }
    };
}
