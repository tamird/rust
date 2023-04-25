#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

trait Type {
    const SIZE: usize;
}
enum Copied<'a, T: Type>
where
    [(); T::SIZE]:,
{
    Borrowed(&'a [u8; T::SIZE]),
    Owned([u8; T::SIZE]),
}
struct Pointer<T> {
    _phantom: core::marker::PhantomData<T>,
}
struct NonNullPointer<T> {
    _phantom: core::marker::PhantomData<T>,
}
impl<T> Type for NonNullPointer<T> {
    const SIZE: usize = Pointer::<T>::SIZE;
}
impl<T> Type for Pointer<T> {
    const SIZE: usize = 8;
}
impl<'a, T: Type> Copied<'a, Pointer<T>>
where
    [(); <NonNullPointer<T>>::SIZE]:,
    [(); <Pointer<T>>::SIZE]:,
    [(); T::SIZE]:,
{
}

fn main() {}
