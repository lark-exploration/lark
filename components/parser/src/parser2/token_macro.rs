#[doc(hidden)]
#[macro_use]
macro_rules! token_impl {
    {
        @tokens = { }
        @list = [ $({ $($token:tt)* })* ]
    } => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
        pub enum Token {
            $(
                $($token)*,
            )*
        }
    };

    {
        @tokens = { $name:ident $($rest:tt)* }
        @list = $list:tt
    } => {
        token_impl! {
            @tokens = { $($rest)* }
            @list = $list
            @name = $name
        }
    };

    {
        @tokens = { , $($rest:tt)* }
        @list = [ $($list:tt)* ]
        @name = $name:ident
    } => {
        token_impl! {
            @tokens = { $($rest)* }
            @list = [ $($list)* { $name } ]
        }
    };

    {
        @tokens = { : String , $($rest:tt)* }
        @list = [ $($list:tt)* ]
        @name = $name:ident
    } => {
        token_impl! {
            @tokens = { $($rest)* }
            @list = [ $($list)* { $name(lark_string::global::GlobalIdentifier) } ]
        }
    };

    {
        @tokens = { : $ty:ty , $($rest:tt)* }
        @list = [ $($list:tt)* ]
        @name = $name:ident
    } => {
        token_impl! {
            @tokens = { $($rest)* }
            @list = [ $($list)* { $name($ty) } ]
        }
    };
}

macro_rules! token {
    ($($rest:tt)*) => {
        token_impl! {
            @tokens = { $($rest)* }
            @list = [ ]
        }
    };
}
