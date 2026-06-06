#![doc = include_str!("../README.md")]
#![no_std]

#[cfg(feature = "use_with_prudent_only")]
#[doc(hidden)]
pub const CARGO_PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "use_with_prudent_only")]
#[macro_export]
#[doc(hidden)]
macro_rules! potentially_check_prudent_version {
    () => {{
        ::prudent::backend::assert_version($crate::CARGO_PKG_VERSION);
    }};
}
#[cfg(not(feature = "use_with_prudent_only"))]
#[macro_export]
#[doc(hidden)]
macro_rules! potentially_check_prudent_version {
    () => {{}};
}

/// Ensure that the given code compiles if it were "final", so if there were no code after it
/// (except for `unreachable!()` or similar way to invoke panic.) That means the given code MAY MOVE
/// any outer values if it wishes. The given code will not be executed.
#[macro_export]
#[doc(hidden)]
macro_rules! ensure_compiles {
    ($( $code:tt )*) => {
        {
            if false {
                let _ = {
                    $( $code )*
                };
                ::core::unreachable!();
            }
        }
    }
}

#[macro_export]
macro_rules! unsafe_fn {
    ( $f:expr; $( $arg:expr ),+ ) => {
        /* Enclosed in (...) and NOT in {...}. Why? Because the later does NOT work if the result is
           an array/slice and then it's accessed with an index suffix `[usize_idx]``.
        */
        (
            /* Enclosed in a block, so that
               1. the result can be used as a value in an outer expression, and
               2. local variables don't conflict with the outer scope
            */
            {
                $crate::potentially_check_prudent_version!();
                /* Ensure that $fn (the expression itself) and any arguments (expressions) don't
                   include any unsafe code/calls/casts on their  own without their own `unsafe{...}`
                   block(s).
                */
                let (tuple_tree, fun) = ($crate::unsafe_fn_internal_build_tuple_tree!{ $($arg),+ }, $f);

                $crate::unsafe_fn_internal_build_accessors_and_call! {
                    fun,
                    tuple_tree,
                    ( $( $arg ),* ),
                    (0)
                }
            }
        )
    };
    ($f:expr) => {
        ({
            $crate::potentially_check_prudent_version!();
            /* Ensure that $fn (the expression itself) doesn't include any unsafe code/calls/casts
               on its own without its own `unsafe{...}` block(s):
            */
            let fun = $f;
            let result = unsafe {
                fun()
            };
            result
        })
    };
}
//----------------------

/// INTERNAL. Do NOT use directly - subject to change.
#[doc(hidden)]
#[macro_export]
macro_rules! unsafe_fn_internal_build_tuple_tree {
    // Construct the tuple_tree. Recursive:
    ( $first:expr, $($rest:expr),+ ) => {
        (
            $first, $crate::unsafe_fn_internal_build_tuple_tree!{ $($rest),+ }
        )
    };
    ( $last:expr) => {
        ($last,)
    };
}

/// INTERNAL. Do NOT use directly - subject to change.
#[doc(hidden)]
#[macro_export]
macro_rules! unsafe_fn_internal_build_accessors_and_call {
    // Access tuple_tree parts and get ready to call the function:
    ( $fn:expr, $tuple_tree:ident,
     ( $_first_arg:expr, $($other_arg:expr),+ ),
     $( ( $($accessor_part:tt),+
        )
     ),*
    ) => {
        $crate::unsafe_fn_internal_build_accessors_and_call!{
            $fn, $tuple_tree, ( $($other_arg),+ ),
            // Insert a new accessor to the front (left): 0.
            (0),
            $(  // Prepend 1 to each supplied/existing accessor
                 ( 1, $($accessor_part),+ )
            ),*
        }
    };
    // All accessors are ready, so call the function:
    ( $fn:expr, $tuple_tree:ident,
      ( $_last_or_only_arg:expr ),
      $( ( $($accessor_part:tt),+
         )
      ),*
    ) => {
        #[allow(unsafe_code)]
        unsafe {
            $fn( $(
                    $crate::unsafe_fn_internal_access_tuple_tree_field!{ $tuple_tree, $($accessor_part),+ }
                ),*
            )
        }
    };
}

/// INTERNAL. Do NOT use directly - subject to change.
///
/// Expand an accessor group/list to access a field in the tuple_tree.
#[doc(hidden)]
#[macro_export]
macro_rules! unsafe_fn_internal_access_tuple_tree_field {
    ( $tuple_tree:ident, $($accessor_part:tt),* ) => {
        $tuple_tree $(. $accessor_part )*
    };
}
//-------------

#[macro_export]
macro_rules! unsafe_method {
    (
        $self:expr =>. $method:ident
     ) => {
        // See unsafe_fn for why here we enclose in ({}...}) and not just in {...}.
        ({
            $crate::potentially_check_prudent_version!();
            $crate::unsafe_method_assert_unsafe_methods!(
                $self =>. $method =>
            )
        })
     };
    (
        $self:expr =>. $method:ident; $( $arg:expr ),*
     ) => {
        ({
            $crate::potentially_check_prudent_version!();
            $crate::unsafe_method_assert_unsafe_methods!(
                $self =>. $method => $( $arg ),*
            )
        })
    }
}
//----------------------

/// Detect code where `unsafe_method!` is not needed at all. Maybe the method used to be `unsafe`,
/// but not anymore.
///
/// Only on `nightly` toolchain and only with `assert_unsafe_methods` feature enabled.
#[macro_export]
#[doc(hidden)]
macro_rules! unsafe_method_assert_unsafe_methods {
    (
        $self:expr =>. $method:ident => $( $arg:expr ),*
     ) => {
        ({// @TODO remove this pair of ({ ... )} and unindent the inner code (??):
            $crate::ensure_compiles! {
                /*
                // "Make" an owned_receiver, an instance/owned value of the same type as $self. (Of
                // course, the instance is invalid - this is for compile-time checks only, hence `if
                // false {...}`.)
                //
                // Then we simulate invocation of the given method inside `unsafe {...}`, BUT
                // without evaluating the given $self expression inside that same `unsafe {...}`
                // block, so that we isolate/catch any `unsafe` code in $self.
                //
                // We **cannot** just move/take/assign $self by value, in case it's a non-`Copy`
                // `static` variable (or a deref of a non-`Copy` raw pointer). See also comments in
                // unsafe_method_internal_build_accessors_check_args_call.
                */
                let owned_receiver = {
                    let rref = &( $self );
                    ::prudent::backend::shared_to_owned( rref )
                };
                #[allow(unused_mut)] // in case the method takes &mut self, or &self.
                let mut owned_receiver = owned_receiver;

                if false {}//$crate::code_assert_unsafe_methods!(owned_receiver =>. $method => $( $arg ),*);
                let _ = unsafe { owned_receiver. $method( $( $arg ),* ) };
            }
            $crate::unsafe_method_internal_check_args_etc!(
                $self, $method $(, $arg )*
            )
        })
     }
}

#[doc(hidden)]
#[macro_export]
macro_rules! unsafe_method_internal_check_args_etc {
    (
        $self:expr, $fn:ident $(, $arg:expr )+
     ) => {({
                let tuple_tree =
                    $crate::unsafe_fn_internal_build_tuple_tree!{ $($arg),+ };

                $crate::unsafe_method_internal_build_accessors_check_args_call! {
                    $self,
                    $fn,
                    tuple_tree,
                    ( $( $arg ),* ),
                    (0)
                }
    })};
    (
        $self:expr, $fn:ident
     ) => {({
                #[allow(unsafe_code)]
                let result = unsafe { $self. $fn () };
                result
    })};
}

#[doc(hidden)]
#[macro_export]
macro_rules! unsafe_method_internal_build_accessors_check_args_call {
    // Access tuple_tree parts and get ready to call the method:
    (
     $self:expr, $fn:ident, $tuple_tree:ident,
     ( $_first_arg:expr, $($other_arg:expr),+ ),
     $( ( $($accessor_part:tt),+
        )
     ),*
    ) => {
        $crate::unsafe_method_internal_build_accessors_check_args_call!{
            $self, $fn, $tuple_tree, ( $($other_arg),+ ),
            // Insert a new accessor to the front (left): 0.
            (0),
            $(  // Prepend 1 to each supplied/existing accessor
                 ( 1, $($accessor_part),+ )
            ),*
        }
    };
    // All accessors are ready. Call the function:
    (
     $self:expr, $fn:ident, $tuple_tree:ident,
      ( $_last_or_only_arg:expr ),
      $( ( $($accessor_part:tt),+
         )
      ),*
    ) => {({
        #[allow(unsafe_code)]
        let result = unsafe {
            // Unlike arguments, we can NOT store result of $self expression in a variable, because
            // - it would be moved, but a method with receiver by reference `&self` or `&mut self`
            // does NOT move the instance it's called on. Also,
            // - if Self were `Copy`, then `&self` or `&mut self` reference would not point to the
            //   original instance! (Plus extra stack used, plus lifetimes issues.)
            // - it could be a non-Copy **static** variable, which cannot be moved.
            $self. $fn( $(
                    $crate::unsafe_fn_internal_access_tuple_tree_field!{ $tuple_tree, $($accessor_part),+ }
                ),*
            )
        };
        result
    })};
}
//-------------

#[macro_export]
macro_rules! unsafe_static_set {
    //@TODO?? #stat:ident
    ($stat:path, $val:expr) => {{
        $crate::potentially_check_prudent_version!();
        $crate::ensure_compiles! {
            let _ = $val;
        }
        unsafe {
            $stat = $val;
        }
    }};

    // $suffix is for example an array index, or a (sub)field
    ($stat:ident { $( $suffix:tt )* } $val:expr) => {{}};
    ($stat:path { $( $suffix:tt )* } $val:expr) => {{
        $crate::potentially_check_prudent_version!();
        $crate::ensure_compiles! {
            let mptr = &raw mut $stat;
            let mref = unsafe { &mut *mptr };
        }
        //@TODO
    }};
}

// @TODO unsafe_static_get

// @TODO unsafe_static_ref
//
//  #[allow(static_mut_refs)]
//  let _r = unsafe { &S };
//
// @TODO unsafe_static_mut
//
//    #[allow(static_mut_refs)]
//    let _m = unsafe { &mut S };

#[macro_export]
macro_rules! unsafe_union_get {
    () => {
        //@TODO
    };
}
#[macro_export]
macro_rules! unsafe_union_set {
    () => {
        //@TODO
    };
}
#[macro_export]
macro_rules! unsafe_union_ref {
    () => {
        //@TODO
    };
}
#[macro_export]
macro_rules! unsafe_union_mut {
    () => {
        //@TODO
    };
}

#[macro_export]
macro_rules! unsafe_ref {
    ($ptr:expr) => {
        // See unsafe_fn for why here we enclose in ({}...}) and not just in {...}.
        // @TODO test with array access (index) right of the macro invocation
        ({
            $crate::potentially_check_prudent_version!();
            let ptr: *const _ = $ptr;
            unsafe { &*ptr }
        })
    };
    ($ptr:expr, $lifetime:lifetime) => {
        ({
            $crate::potentially_check_prudent_version!();
            let ptr: *const _ = $ptr;
            unsafe { &*ptr as &$lifetime _ }
        })
    };
    ($ptr:expr, $ptr_type:ty) => {
        ({
            $crate::potentially_check_prudent_version!();
            let ptr = $ptr;
            let ptr = ptr as *const $ptr_type;
            unsafe { &*ptr }
        })
    };
    ($ptr:expr, $ptr_type:ty, $lifetime:lifetime) => {
        ({
            $crate::potentially_check_prudent_version!();
            let ptr = $ptr;
            let ptr = ptr as *const $ptr_type;
            unsafe { &*ptr as &$lifetime _ }
            })
    };
}

#[macro_export]
macro_rules! unsafe_mut {
    ($ptr:expr) => {
        // See unsafe_fn for why here we enclose in ({}...}) and not just in {...}.
        ({
            $crate::potentially_check_prudent_version!();
            let ptr: *mut _ = $ptr;
            unsafe { &mut *ptr }
        })
    };
    ($ptr:expr, $lifetime:lifetime) => {
        ({
            $crate::potentially_check_prudent_version!();
            let ptr: *mut _ = $ptr;
            unsafe { &mut *ptr as &$lifetime mut _}
        })
    };
    ($ptr:expr, $ptr_type:ty) => {
        ({
            $crate::potentially_check_prudent_version!();
            let ptr = $ptr;
            let ptr = ptr as *mut $ptr_type;
            unsafe { &mut *ptr}
        })
    };
    ($ptr:expr, $ptr_type:ty, $lifetime:lifetime) => {
        ({
            $crate::potentially_check_prudent_version!();
            let ptr = $ptr;
            let ptr = ptr as *mut $ptr_type;
            unsafe { &mut *ptr as &$lifetime mut _}
        })
    };
}

#[macro_export]
macro_rules! unsafe_val {
    ($ptr:expr) => {
        // See unsafe_fn for why here we enclose in ({}...}) and not just in {...}.
        ({
            $crate::potentially_check_prudent_version!();
            let ptr: *const _ = $ptr;
            ::prudent::backend::expect_copy_ptr(ptr);
            unsafe { *ptr }
        })
    };
    ($ptr:expr => $ptr_type:ty) => {
        ({
            $crate::potentially_check_prudent_version!();
            let ptr = $ptr;
            let ptr = ptr as *const $ptr_type;
            ::prudent::backend::expect_copy_ptr(ptr);
            unsafe { *ptr }
        })
    };
}

/*
-nightly version only
https://doc.rust-lang.org/std/keyword.use.html#ergonomic-clones
https://doc.rust-lang.org/std/clone/trait.UseCloned.html


#[macro_export]
macro_rules! unsafe_use {
    ($ptr:expr) => {{
        let ptr = $ptr;
        unsafe { ( *ptr ).use }
    }};
    ($ptr:expr, $ptr_type:ty) => {{
        let ptr = $ptr as $ptr_type;
        unsafe { ( *ptr ).use }
    }};
}*/

#[macro_export]
macro_rules! unsafe_set {
    ($ptr:expr, $value:expr) => {{
        $crate::potentially_check_prudent_version!();
        $crate::ensure_compiles! {
            let _: *mut _ = $ptr;
            let _ = $value;
        }
        unsafe {
            *$ptr = $value;
        }
    }};
}
