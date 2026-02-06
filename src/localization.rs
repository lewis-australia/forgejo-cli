//include!(concat!(env!("OUT_DIR"), "/locale-generated.rs"));
include!("../locale-generated.rs");

mod functions {
    pub mod en_us {
        use super::super::FluentValue;
        pub struct STYLE<A0: FluentValue> {
            pub __positional: (A0,),
        }

        impl FluentValue for STYLE<&str> {
            fn display(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Ok(())
            }

            fn matches_string(&self, s: &str) -> bool {
                false
            }
        }
    }

    pub mod tok {}
}

pub const LOCALE: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| std::env::var("LANG").unwrap_or_else(|_| "C".into()));

pub trait FluentValue {
    fn display(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;

    fn display_str(&self) -> String {
        display_fn(|f| self.display(f)).to_string()
    }

    fn matches_string(&self, s: &str) -> bool;
}

impl FluentValue for u32 {
    fn display(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <u32 as std::fmt::Display>::fmt(self, f)
    }

    fn matches_string(&self, s: &str) -> bool {
        match s {
            "zero" => *self == 0,
            "one" => *self == 1,
            _ => false,
        }
    }
}

impl FluentValue for f64 {
    fn display(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <f64 as std::fmt::Display>::fmt(self, f)
    }

    fn matches_string(&self, s: &str) -> bool {
        match s {
            "zero" => *self == 0.0,
            "one" => *self == 1.0,
            _ => false,
        }
    }
}

impl FluentValue for str {
    fn display(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <str as std::fmt::Display>::fmt(self, f)
    }

    fn matches_string(&self, s: &str) -> bool {
        self == s
    }
}

impl<'a, T: FluentValue + ?Sized> FluentValue for &'a T {
    fn display(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::display(self, f)
    }

    fn matches_string(&self, s: &str) -> bool {
        T::matches_string(self, s)
    }
}

pub struct DisplayFn<F>(pub F);

pub fn display_fn<F: Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result>(f: F) -> DisplayFn<F> {
    DisplayFn(f)
}

impl<F> std::fmt::Display for DisplayFn<F>
where
    F: Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self.0)(f)
    }
}

#[macro_export]
macro_rules! ftl_format {
    ($msg_id:ident) => {
        $crate::ftl_format!($msg_id,)
    };
    ($msg_id:ident, $($var_name:ident = $var_val:expr),*) => {
        {
            use std::fmt::Write;
            let mut s = String::new();
            $crate::ftl_write!(&mut s, $msg_id, $($var_name = $var_val),*).expect("failed to format localized text");
            s
        }
    }
}

#[macro_export]
macro_rules! ftl_write {
    ($writer:expr, $msg_id:ident) => {
        $crate::ftl_write!($writer, $msg_id,)
    };
    ($writer:expr, $msg_id:ident, $($var_name:ident = $var_val:expr),*) => {
        {
            write!($writer, "{}", $crate::localization::display_fn(|f| {
                $crate::localization::$msg_id(
                    f,
                    $crate::localization::args::$msg_id {
                        $(
                            $var_name: $var_val,
                        ),*
                    }
                ).map_err(|_| std::fmt::Error)?;
                Ok(())
            }))
        }
    }
}

#[macro_export]
macro_rules! ftl_print {
    ($msg_id:ident) => {
        $crate::ftl_print!($msg_id,)
    };
    ($msg_id:ident, $($var_name:ident = $var_val:expr),*) => {
        {
            $crate::ftl_write!(&mut std::io::stdout(), $msg_id, $($var_name = $var_val),*).expect("failed to print localized text");
        }
    }
}

#[macro_export]
macro_rules! ftl_println {
    ($msg_id:ident) => {
        $crate::ftl_println!($msg_id,)
    };
    ($msg_id:ident, $($var_name:ident = $var_val:expr),*) => {
        {
            let mut stdout = std::io::stdout();
            use std::io::Write;
            $crate::ftl_write!(&mut stdout, $msg_id, $($var_name = $var_val),*).expect("failed to print localized text");
            writeln!(&mut stdout).expect("failed to write newline");
        }
    }
}

#[macro_export]
macro_rules! ftl_eprint {
    ($msg_id:ident) => {
        $crate::ftl_eprint!($msg_id,)
    };
    ($msg_id:ident, $($var_name:ident = $var_val:expr),*) => {
        {
            let mut stderr = $crate::localization::WriterCompat();
            $crate::ftl_write!(&mut std::io::stderr(), $msg_id, $($var_name = $var_val),*).expect("failed to print localized text");
        }
    }
}

#[macro_export]
macro_rules! ftl_eprintln {
    ($msg_id:ident) => {
        $crate::ftl_eprintln!($msg_id,)
    };
    ($msg_id:ident, $($var_name:ident = $var_val:expr),*) => {
        {
            use std::io::Write;
            let mut stderr = std::io::stderr();
            $crate::ftl_write!(&mut stderr, $msg_id, $($var_name = $var_val),*).expect("failed to print localized text");
            writeln!(&mut stderr).expect("failed to write newline");
        }
    }
}
