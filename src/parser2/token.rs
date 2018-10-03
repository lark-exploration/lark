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

        impl $crate::parser::ast::DebugModuleTable for Token {
            fn debug(&self, f: &mut std::fmt::Formatter<'_>, _table: &'table $crate::parser::ModuleTable) -> std::fmt::Result {
                write!(f, "{:?}", self)
            }
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
            @list = [ $($list)* { $name($crate::parser::program::StringId) } ]
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
