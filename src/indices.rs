#[macro_use]
macro_rules! index_type {
    ($(#[$attr:meta])* $visibility:vis struct $name:ident { .. }) => {
        index_type! { @with {
            attrs[$($attr)*],
            visibility[$visibility],
            name[$name],
            max[std::u32::MAX],
            debug_name[stringify!($name)],
        } }
    };

    (@with {
        attrs[$($attrs:meta)*],
        visibility[$visibility:vis],
        name[$name:ident],
        max[$max:expr],
        debug_name[$debug_name:expr],
    }) => {
        #[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
        $(#[$attrs])*
        $visibility struct $name {
            private: u32
        }

        impl $name {
            $visibility const fn new(index: usize) -> Self {
                // This is a wacky assert that is compatible with a
                // const fn.  It will evaluate to an out-of-bounds
                // access if `index >= $max`.
                Self { private: [index as u32][(index < ($max as usize)) as usize] }
            }

            $visibility const fn from_u32(index: u32) -> Self {
                // This is a wacky assert that is compatible with a
                // const fn.  It will evaluate to an out-of-bounds
                // access if `index >= $max`.
                Self { private: [index][(index < $max) as usize] }
            }

            $visibility fn as_u32(self) -> u32 {
                self.private
            }

            $visibility fn as_usize(self) -> usize {
                self.private as usize
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

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.as_usize())
                    .finish()
            }
        }
    };
}
