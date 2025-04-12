#![allow(missing_debug_implementations)]
#![unstable(feature = "fmt_internals", reason = "internal to format_args!", issue = "none")]

//! All types and methods in this file are used by the compiler in
//! the expansion/lowering of format_args!().
//!
//! Do not modify them without understanding the consequences for the format_args!() macro.

use super::*;
use crate::hint::unreachable_unchecked;

#[lang = "format_placeholder"]
#[derive(Copy, Clone)]
pub struct Placeholder {
    pub position: usize,
    pub flags: u32,
    pub precision: Count,
    pub width: Count,
}

/// Used by [width](https://doc.rust-lang.org/std/fmt/#width)
/// and [precision](https://doc.rust-lang.org/std/fmt/#precision) specifiers.
#[lang = "format_count"]
#[derive(Copy, Clone)]
pub enum Count {
    /// Specified with a literal number, stores the value
    Is(u16),
    /// Specified using `$` and `*` syntaxes, stores the index into `args`
    Param(usize),
    /// Not specified
    Implied,
}

trait FormatThunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

#[derive(Copy, Clone)]
enum ArgumentType<'a> {
    Placeholder(&'a dyn FormatThunk),
    Count(u16),
}

/// This struct represents a generic "argument" which is taken by format_args!().
///
/// This can be either a placeholder argument or a count argument.
/// * A placeholder argument contains a function to format the given value. At
///   compile time it is ensured that the function and the value have the correct
///   types, and then this struct is used to canonicalize arguments to one type.
///   Placeholder arguments are essentially an optimized partially applied formatting
///   function, equivalent to `exists T.(&T, fn(&T, &mut Formatter<'_>) -> Result`.
/// * A count argument contains a count for dynamic formatting parameters like
///   precision and width.
#[lang = "format_argument"]
#[derive(Copy, Clone)]
pub struct Argument<'a> {
    ty: ArgumentType<'a>,
}

macro_rules! implement_argument_constructor {
    ($trait:ident, $operand:expr, $fmt:item) => {{
        #[repr(transparent)]
        struct Wrapper<T>(T);

        // SAFETY: `Wrapper<T>` has the same memory layout as `T` due to #[repr(transparent)].
        let thunk = unsafe { mem::transmute::<&T, &Wrapper<T>>($operand) };

        impl<T: $trait> FormatThunk for Wrapper<T> {
            #[inline]
            $fmt
        }

        Self::new(thunk)
    }};

    ($trait:ident, $operand:expr) => {
        implement_argument_constructor!(
            $trait,
            $operand,
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let Self(inner) = self;
                inner.fmt(f)
            }
        )
    };
}

impl Argument<'_> {
    #[inline]
    fn new<'a>(x: &'a dyn FormatThunk) -> Argument<'a> {
        Argument { ty: ArgumentType::Placeholder(x) }
    }

    #[inline]
    pub fn new_display<T: Display>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(Display, x)
    }
    #[inline]
    pub fn new_debug<T: Debug>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(Debug, x)
    }
    #[inline]
    pub fn new_debug_noop<T: Debug>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(
            Debug,
            x,
            fn fmt(&self, _: &mut Formatter<'_>) -> Result {
                Ok(())
            }
        )
    }
    #[inline]
    pub fn new_octal<T: Octal>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(Octal, x)
    }
    #[inline]
    pub fn new_lower_hex<T: LowerHex>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(LowerHex, x)
    }
    #[inline]
    pub fn new_upper_hex<T: UpperHex>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(UpperHex, x)
    }
    #[inline]
    pub fn new_pointer<T: Pointer>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(Pointer, x)
    }
    #[inline]
    pub fn new_binary<T: Binary>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(Binary, x)
    }
    #[inline]
    pub fn new_lower_exp<T: LowerExp>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(LowerExp, x)
    }
    #[inline]
    pub fn new_upper_exp<T: UpperExp>(x: &T) -> Argument<'_> {
        implement_argument_constructor!(UpperExp, x)
    }

    #[inline]
    #[track_caller]
    pub const fn from_usize(x: &usize) -> Argument<'_> {
        if *x > u16::MAX as usize {
            panic!("Formatting argument out of range");
        }
        Argument { ty: ArgumentType::Count(*x as u16) }
    }

    /// Format this placeholder argument.
    ///
    /// # Safety
    ///
    /// This argument must actually be a placeholder argument.
    #[inline]
    pub(super) unsafe fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.ty {
            ArgumentType::Placeholder(thunk) => thunk.fmt(f),
            // SAFETY: the caller promised this.
            ArgumentType::Count(_) => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(super) const fn as_u16(&self) -> Option<u16> {
        match self.ty {
            ArgumentType::Count(count) => Some(count),
            ArgumentType::Placeholder { .. } => None,
        }
    }

    /// Used by `format_args` when all arguments are gone after inlining,
    /// when using `&[]` would incorrectly allow for a bigger lifetime.
    ///
    /// This fails without format argument inlining, and that shouldn't be different
    /// when the argument is inlined:
    ///
    /// ```compile_fail,E0716
    /// let f = format_args!("{}", "a");
    /// println!("{f}");
    /// ```
    #[inline]
    pub const fn none() -> [Self; 0] {
        []
    }
}

/// This struct represents the unsafety of constructing an `Arguments`.
/// It exists, rather than an unsafe function, in order to simplify the expansion
/// of `format_args!(..)` and reduce the scope of the `unsafe` block.
#[lang = "format_unsafe_arg"]
pub struct UnsafeArg {
    _private: (),
}

impl UnsafeArg {
    /// See documentation where `UnsafeArg` is required to know when it is safe to
    /// create and use `UnsafeArg`.
    #[inline]
    pub const unsafe fn new() -> Self {
        Self { _private: () }
    }
}

/// Used by the format_args!() macro to create a fmt::Arguments object.
#[doc(hidden)]
#[unstable(feature = "fmt_internals", issue = "none")]
#[rustc_diagnostic_item = "FmtArgumentsNew"]
impl<'a> Arguments<'a> {
    #[inline]
    pub const fn new_const<const N: usize>(pieces: &'a [&'static str; N]) -> Self {
        const { assert!(N <= 1) };
        Arguments { pieces, fmt: None, args: &[] }
    }

    /// When using the format_args!() macro, this function is used to generate the
    /// Arguments structure.
    ///
    /// This function should _not_ be const, to make sure we don't accept
    /// format_args!() and panic!() with arguments in const, even when not evaluated:
    ///
    /// ```compile_fail,E0015
    /// const _: () = if false { panic!("a {}", "a") };
    /// ```
    #[inline]
    pub fn new_v1<const P: usize, const A: usize>(
        pieces: &'a [&'static str; P],
        args: &'a [rt::Argument<'a>; A],
    ) -> Arguments<'a> {
        const { assert!(P >= A && P <= A + 1, "invalid args") }
        Arguments { pieces, fmt: None, args }
    }

    /// Specifies nonstandard formatting parameters.
    ///
    /// An `rt::UnsafeArg` is required because the following invariants must be held
    /// in order for this function to be safe:
    /// 1. The `pieces` slice must be at least as long as `fmt`.
    /// 2. Every `rt::Placeholder::position` value within `fmt` must be a valid index of `args`.
    /// 3. Every `rt::Count::Param` within `fmt` must contain a valid index of `args`.
    ///
    /// This function should _not_ be const, to make sure we don't accept
    /// format_args!() and panic!() with arguments in const, even when not evaluated:
    ///
    /// ```compile_fail,E0015
    /// const _: () = if false { panic!("a {:1}", "a") };
    /// ```
    #[inline]
    pub fn new_v1_formatted(
        pieces: &'a [&'static str],
        args: &'a [rt::Argument<'a>],
        fmt: &'a [rt::Placeholder],
        _unsafe_arg: rt::UnsafeArg,
    ) -> Arguments<'a> {
        Arguments { pieces, fmt: Some(fmt), args }
    }
}
