use core::marker::PhantomData;

pub(crate) type QueryShapeMarker<Q, Filter, Shape> = PhantomData<fn() -> (Q, Filter, Shape)>;

macro_rules! define_arities {
    ($($arity:ident),+ $(,)?) => {
        $(
            #[doc(hidden)]
            pub struct $arity;
        )+
    };
}

define_arities!(
    Arity1, Arity2, Arity3, Arity4, Arity5, Arity6, Arity7, Arity8, Arity9, Arity10, Arity11,
    Arity12, Arity13, Arity14, Arity15, Arity16,
);

/// Describes the single argument exposed by a width-one query callback.
#[doc(hidden)]
pub trait Args1 {
    type A;

    fn split(self) -> (Self::A,);
}

impl<A> Args1 for A {
    type A = A;

    #[inline(always)]
    fn split(self) -> (Self::A,) {
        (self,)
    }
}

macro_rules! define_tuple_args {
    ($Args:ident, $($Assoc:ident),+ $(,)?) => {
        #[doc(hidden)]
        pub trait $Args {
            $(type $Assoc;)+

            fn split(self) -> ($(Self::$Assoc,)+);
        }

        impl<$($Assoc),+> $Args for ($($Assoc,)+) {
            $(type $Assoc = $Assoc;)+

            #[inline(always)]
            fn split(self) -> ($(Self::$Assoc,)+) {
                self
            }
        }
    };
}

define_tuple_args!(Args2, A, B);
define_tuple_args!(Args3, A, B, C);
define_tuple_args!(Args4, A, B, C, D);
define_tuple_args!(Args5, A, B, C, D, E);
define_tuple_args!(Args6, A, B, C, D, E, F);
define_tuple_args!(Args7, A, B, C, D, E, F, G);
define_tuple_args!(Args8, A, B, C, D, E, F, G, H);
define_tuple_args!(Args9, A, B, C, D, E, F, G, H, I);
define_tuple_args!(Args10, A, B, C, D, E, F, G, H, I, J);
define_tuple_args!(Args11, A, B, C, D, E, F, G, H, I, J, K);
define_tuple_args!(Args12, A, B, C, D, E, F, G, H, I, J, K, L);
define_tuple_args!(Args13, A, B, C, D, E, F, G, H, I, J, K, L, M);
define_tuple_args!(Args14, A, B, C, D, E, F, G, H, I, J, K, L, M, N);
define_tuple_args!(Args15, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
define_tuple_args!(Args16, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
