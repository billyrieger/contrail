// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Low-level memory management.
//!
//! # Warning
//!
//! `Pointer` and `ArrayPointer` are only usable with the `Memory` from the finished
//! `MemoryBuilder` used to create the pointer (or a clone of the `Memory`).  It is unsafe behavior
//! to use a pointer with any `Memory` other than what initialized it.
use std::{fmt, marker::PhantomData};

/// Anything that can be converted to or from a fixed-length byte slice.
///
/// In theory, there could be a blanket implementation of `Bytes` for types that are `Copy +
/// 'static`. Unfortunately such an implementation is impossible until [this Rust
/// issue](https://github.com/rust-lang/rust/issues/43408) is resolved. For now, `Bytes` is only
/// implemented for the following primitive types:
///
/// - `i8`, `i16`, `i32`, `i64`, `i128`, `isize`
/// - `u8`, `u16`, `u32`, `u64`, `u128`, `usize`
/// - `f32`, `f64`
/// - `char`
/// - `bool`
/// - `()`
///
/// # Deriving `Bytes`
///
/// `Bytes` can be derived on custom data types with `#[derive(Bytes)]`.  To use this feature,
/// `#[macro_use] extern crate contrail` must be in the crate root.
///
/// Since `Bytes: Copy`, it's usually necessary to derive `Clone` and `Copy` as well.
///
/// ```
/// # #[macro_use] extern crate contrail;
/// #
/// #[derive(Bytes, Clone, Copy)]
/// enum Flavor {
///     Up,
///     Charm,
///     Top,
///     Down,
///     Strange,
///     Bottom,
/// }
///
/// #[derive(Bytes, Clone, Copy)]
/// struct Wrapper {
///     inner: Result<[char; 5], Option<(i128, &'static str)>>,
/// }
/// ```
///
/// ## Limitations
///
/// Due to the Rust issue mentioned above, using generic parameters is disallowed when deriving
/// `Bytes`. This includes type parameters as well as lifetimes (although generic lifetimes would
/// be disallowed anyway since `Bytes: 'static`):
///
/// ```compile_fail
/// # #[macro_use] extern crate contrail;
/// #
/// // doesn't compile (but maybe it will one day)
/// #[derive(Bytes, Clone, Copy)]
/// struct Wrapper<T>
/// where
///     T: Copy + 'static,
/// {
///     inner: T,
/// }
/// ```
///
/// ```compile_fail
/// # #[macro_use] extern crate contrail;
/// #
/// // doesn't compile (and shouldn't)
/// #[derive(Bytes, Clone, Copy)]
/// struct StringRef<'a> {
///     inner: &'a String,
/// }
/// ```
///
/// # Examples
///
/// ```
/// use contrail::mem::Bytes;
///
/// let mut bytes = [0; 2];
/// let data: u16 = 0xCAFE;
///
/// unsafe { data.write_bytes(&mut bytes) };
///
/// assert_eq!(unsafe { u16::read_bytes(&bytes) }, 0xCAFE);
/// ```
pub trait Bytes: Copy + 'static {
    /// The size of `Self` in bytes.
    const LENGTH: usize;

    /// Reads a value of type `Self` from the byte slice.
    ///
    /// The caller must guarantee that `bytes.len() == Self::LENGTH` and that the byte slice
    /// represents a valid value of type `Self`. Really the only way to be sure of this is to write
    /// a valid value to the byte slice beforehand.
    unsafe fn read_bytes(bytes: &[u8]) -> Self;

    /// Writes a copy of `self` to the byte slice.
    ///
    /// The caller must guarantee that `bytes.len() == Self::LENGTH`.
    unsafe fn write_bytes(self, bytes: &mut [u8]);
}

/// A fixed-size chunk of bytes that can be accessed and updated using pointers.
///
/// `Memory` has no methods itself.  To create `Memory`, use a
/// [`MemoryBuilder`](crate::mem::MemoryBuilder).  All operations that read from or write to the
/// memory are performed with a [`Pointer`](Pointer) or an [`ArrayPointer`](ArrayPointer).
///
/// # Warning
///
/// It is unsafe behavior to use pointers with `Memory` other than the `Memory` from the
/// `MemoryBuilder` used to initialize the pointer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Memory {
    bytes: Vec<u8>,
}

/// A growable chunk of bytes that can be built into `Memory`.
///
/// Values and arrays are not added to the `MemoryBuilder` directly; rather, the `new` methods for
/// [`Pointer`](Pointer::new) and [`ArrayPointer`](ArrayPointer::new) take `&mut MemoryBuilder` as
/// the first parameter.  After everything is added to the `MemoryBuilder`, the
/// [`finish`](MemoryBuilder::finish) method consumes the `MemoryBuilder` and creates a `Memory`,
/// which is usable with the pointers created with the original `MemoryBuilder`.
///
/// # Warning
///
/// It is unsafe behavior to use pointers with `Memory` other than the `Memory` from the
/// `MemoryBuilder` used to initialize the pointer.
///
/// # Examples
///
/// ```
/// use contrail::mem::{MemoryBuilder, Pointer};
///
/// let mut builder = MemoryBuilder::new();
/// let pointer = Pointer::new(&mut builder, 'R');
/// let mut memory = builder.finish();
///
/// // the pointer is usable with the memory after the builder finishes
/// assert_eq!(pointer.get(&memory), 'R');
/// ```
#[derive(Debug, Default, Eq, PartialEq)]
pub struct MemoryBuilder {
    bytes: Vec<u8>,
}

impl MemoryBuilder {
    /// Creates a new empty `MemoryBuilder`.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::MemoryBuilder;
    ///
    /// let mut builder = MemoryBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self { bytes: vec![] }
    }

    /// Consumes the `MemoryBuilder` to create a `Memory`.
    ///
    /// After calling `finish`, all pointers created using the `MemoryBuilder` can safely read to
    /// and write from the returned `Memory`.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{ArrayPointer, MemoryBuilder, Pointer};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let pointer = Pointer::new(&mut builder, 42);
    /// let array = ArrayPointer::new(&mut builder, &[0, 1, 1, 2, 3, 5, 8]);
    /// let memory = builder.finish();
    ///
    /// // `pointer` and `array` are usable now
    /// assert_eq!(pointer.get(&memory), 42);
    /// assert_eq!(array.get(&memory, 5), 5);
    /// ```
    pub fn finish(self) -> Memory {
        Memory { bytes: self.bytes }
    }
}

/// A reference to a value in memory.
///
/// # Warning
///
/// A `Pointer` is only usable with the `Memory` from the `MemoryBuilder` used to create the
/// `Pointer`. Using a `Pointer` with any other memory is considered undefined behavior.
///
/// # Examples
///
/// ```
/// use contrail::mem::{MemoryBuilder, Pointer};
///
/// let mut builder = MemoryBuilder::new();
/// let pointer = Pointer::new(&mut builder, 10);
/// let mut memory = builder.finish();
///
/// assert_eq!(pointer.get(&memory), 10);
///
/// pointer.update(&mut memory, |x| x * 2);
/// assert_eq!(pointer.get(&memory), 20);
/// ```
pub struct Pointer<T> {
    offset: usize,
    phantom: PhantomData<T>,
}

impl<T> Pointer<T>
where
    T: Bytes,
{
    /// Creates a new pointer to the given value in memory.
    ///
    /// The pointer is only usable after the `MemoryBuilder` is finished and `Memory` is created.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{MemoryBuilder, Pointer};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let pointer = Pointer::new(&mut builder, 'b');
    /// let memory = builder.finish();
    ///
    /// // the pointer is usable now
    /// assert_eq!(pointer.get(&memory), 'b');
    /// ```
    pub fn new(builder: &mut MemoryBuilder, val: T) -> Self {
        let offset = builder.bytes.len();

        // create uninitialized memory
        builder.bytes.extend((0..T::LENGTH).map(|_| 0));

        // initialize the memory
        unsafe {
            val.write_bytes(
                builder
                    .bytes
                    .get_unchecked_mut(offset..(offset + T::LENGTH)),
            )
        }

        Self {
            offset,
            phantom: PhantomData,
        }
    }

    /// Gets the value of the pointer from memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{MemoryBuilder, Pointer};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let pi = Pointer::new(&mut builder, 3.14);
    /// let memory = builder.finish();
    ///
    /// assert_eq!(pi.get(&memory), 3.14);
    /// ```
    #[inline(always)]
    pub fn get(self, memory: &Memory) -> T {
        unsafe {
            T::read_bytes(
                memory
                    .bytes
                    .get_unchecked(self.offset..(self.offset + T::LENGTH)),
            )
        }
    }

    /// Sets the value of the pointer in memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{MemoryBuilder, Pointer};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let letter = Pointer::new(&mut builder, 'a');
    /// let mut memory = builder.finish();
    ///
    /// letter.set(&mut memory, 'z');
    /// assert_eq!(letter.get(&memory), 'z');
    /// ```
    #[inline(always)]
    pub fn set(self, memory: &mut Memory, val: T) {
        unsafe {
            val.write_bytes(
                memory
                    .bytes
                    .get_unchecked_mut(self.offset..(self.offset + T::LENGTH)),
            );
        }
    }

    /// Updates the value in memory using the given function.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{MemoryBuilder, Pointer};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let side = Pointer::new(&mut builder, 5);
    /// let mut memory = builder.finish();
    ///
    /// side.update(&mut memory, |x| x * x);
    /// assert_eq!(side.get(&memory), 25);
    /// ```
    #[inline(always)]
    pub fn update(self, memory: &mut Memory, f: impl FnOnce(T) -> T) {
        self.set(memory, f(self.get(memory)));
    }
}

impl<T> Clone for Pointer<T> {
    fn clone(&self) -> Self {
        Self {
            offset: self.offset,
            phantom: PhantomData,
        }
    }
}

impl<T> Copy for Pointer<T> {}

impl<T> fmt::Debug for Pointer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Pointer")
            .field("offset", &self.offset)
            .finish()
    }
}

impl<T> Eq for Pointer<T> {}

impl<T> PartialEq for Pointer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}

/// A reference to an array of values in memory.
///
/// # Warning
///
/// An `ArrayPointer` is only usable with the `Memory` from the `MemoryBuilder` used to create the
/// `ArrayPointer`. Using a `ArrayPointer` with any other memory is considered undefined behavior.
///
/// # Examples
///
/// ```
/// use contrail::mem::{ArrayPointer, MemoryBuilder};
///
/// let mut builder = MemoryBuilder::new();
/// let array = ArrayPointer::new(&mut builder, &[0, 1, 3, 2]);
/// let mut memory = builder.finish();
///
/// assert_eq!(array.len(), 4);
/// assert_eq!(array.get(&memory, 3), 2);
/// ```
pub struct ArrayPointer<T> {
    offset: usize,
    len: usize,
    phantom: PhantomData<T>,
}

impl<T> ArrayPointer<T>
where
    T: Bytes,
{
    /// Creates a new pointer to the given array of values in memory.
    ///
    /// The pointer is only usable after the `MemoryBuilder` is finished and
    /// `Memory` is created.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{ArrayPointer, MemoryBuilder};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let notes = ArrayPointer::new(&mut builder, &['c', 'e', 'g']);
    /// let memory = builder.finish();
    ///
    /// // the pointer is usable now
    /// assert_eq!(notes.get(&memory, 0), 'c');
    /// ```
    pub fn new(builder: &mut MemoryBuilder, vals: &[T]) -> Self {
        let offset = builder.bytes.len();

        // create uninitialized memory
        builder
            .bytes
            .extend((0..(T::LENGTH * vals.len())).map(|_| 0));

        // initialize the memory
        let mut val_offset = offset;
        for val in vals.iter() {
            unsafe {
                val.write_bytes(
                    builder
                        .bytes
                        .get_unchecked_mut(val_offset..(val_offset + T::LENGTH)),
                );
            }
            val_offset += T::LENGTH;
        }

        Self {
            offset,
            len: vals.len(),
            phantom: PhantomData,
        }
    }

    /// Returns the length of the array.
    ///
    /// Note that this function does not take any parameters as the length of the array is
    /// constant.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{ArrayPointer, MemoryBuilder};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let array = ArrayPointer::new(&mut builder, &[1, 2, 3, 4, 5]);
    /// let memory = builder.finish();
    ///
    /// assert_eq!(array.len(), 5);
    /// ```
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the length of the array is zero.
    ///
    /// Note that this function does not take any parameters as the length of the array is
    /// constant.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{ArrayPointer, MemoryBuilder};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let empty = ArrayPointer::<i32>::new(&mut builder, &[]);
    /// let not_empty = ArrayPointer::new(&mut builder, &[5i32]);
    /// let memory = builder.finish();
    ///
    /// assert_eq!(empty.is_empty(), true);
    /// assert_eq!(not_empty.is_empty(), false);
    /// ```
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Gets the value of the given index of the array pointer from memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{ArrayPointer, MemoryBuilder};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let doubles = ArrayPointer::new(&mut builder, &[0, 2, 4, 6]);
    /// let memory = builder.finish();
    ///
    /// assert_eq!(doubles.get(&memory, 2), 4);
    /// ```
    #[inline(always)]
    pub fn get(&self, memory: &Memory, i: usize) -> T {
        assert!(i < self.len);
        let mem_offset = self.offset + i * T::LENGTH;
        unsafe {
            T::read_bytes(
                memory
                    .bytes
                    .get_unchecked(mem_offset..(mem_offset + T::LENGTH)),
            )
        }
    }

    /// Sets the value of the given index of the array pointer in memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{ArrayPointer, MemoryBuilder};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let letters = ArrayPointer::new(&mut builder, &['a', 'b', 'c']);
    /// let mut memory = builder.finish();
    ///
    /// assert_eq!(letters.get(&memory, 1), 'b');
    ///
    /// letters.set(&mut memory, 1, 'z');
    /// assert_eq!(letters.get(&memory, 1), 'z');
    /// ```
    #[inline(always)]
    pub fn set(&self, memory: &mut Memory, i: usize, val: T) {
        assert!(i < self.len);
        let mem_offset = self.offset + i * T::LENGTH;
        unsafe {
            val.write_bytes(
                memory
                    .bytes
                    .get_unchecked_mut(mem_offset..(mem_offset + T::LENGTH)),
            );
        }
    }

    /// Updates the value of the given index in memory using the given function.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{ArrayPointer, MemoryBuilder};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let truth_table = ArrayPointer::new(&mut builder, &[true, true, true, false]);
    /// let mut memory = builder.finish();
    ///
    /// assert_eq!(truth_table.get(&memory, 3), false);
    ///
    /// truth_table.update(&mut memory, 3, |x| !x);
    /// assert_eq!(truth_table.get(&memory, 3), true);
    /// ```
    #[inline(always)]
    pub fn update(&self, memory: &mut Memory, i: usize, f: impl FnOnce(T) -> T) {
        self.set(memory, i, f(self.get(memory, i)));
    }

    /// Swaps the values in memory of two indices of the array pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::mem::{ArrayPointer, MemoryBuilder};
    ///
    /// let mut builder = MemoryBuilder::new();
    /// let digits = ArrayPointer::new(&mut builder, &[3, 4, 1, 1, 5, 9]);
    /// let mut memory = builder.finish();
    ///
    /// assert_eq!(digits.get(&memory, 1), 4);
    /// assert_eq!(digits.get(&memory, 2), 1);
    ///
    /// digits.swap(&mut memory, 1, 2);
    ///
    /// assert_eq!(digits.get(&memory, 1), 1);
    /// assert_eq!(digits.get(&memory, 2), 4);
    /// ```
    #[inline(always)]
    pub fn swap(&self, memory: &mut Memory, i: usize, j: usize) {
        let temp_i = self.get(memory, i);
        self.set(memory, i, self.get(memory, j));
        self.set(memory, j, temp_i);
    }
}

impl<T> Clone for ArrayPointer<T> {
    fn clone(&self) -> Self {
        Self {
            offset: self.offset,
            len: self.len,
            phantom: PhantomData,
        }
    }
}

impl<T> Copy for ArrayPointer<T> {}

impl<T> fmt::Debug for ArrayPointer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ArrayPointer")
            .field("offset", &self.offset)
            .field("len", &self.len)
            .finish()
    }
}

impl<T> Eq for ArrayPointer<T> {}

impl<T> PartialEq for ArrayPointer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset && self.len == other.len
    }
}

macro_rules! impl_bytes_primitive {
    ( $( $T:ty ),* , ) => {
        $(
            impl Bytes for $T {
                const LENGTH: usize = std::mem::size_of::<$T>();

                #[inline(always)]
                unsafe fn read_bytes(bytes: &[u8]) -> $T {
                    // safe assuming that the length of the byte slice is Self::LENGTH.
                    let byte_array = *(bytes.as_ptr() as *const [u8; std::mem::size_of::<$T>()]);
                    // safe assuming that the byte slice represents a valid value of type T.
                    std::mem::transmute::<[u8; std::mem::size_of::<$T>()], $T>(byte_array)
                }

                #[inline(always)]
                unsafe fn write_bytes(self, bytes: &mut [u8]) {
                    // safe for Copy + 'static types
                    let byte_array = std::mem::transmute::<$T, [u8; std::mem::size_of::<$T>()]>(self);
                    // safe assuming that the length of the byte slice is Self::LENGTH.
                    bytes.copy_from_slice(&byte_array);
                }
            }
        )*
    }
}

impl_bytes_primitive! {
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64,
    char,
    bool,
    (),
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRIALS: usize = 100;
    const SEED: [u8; 32] = [42; 32];

    // repeatedly verifies that a random value can be written to and read from a byte slice
    macro_rules! test_bytes {
        ( $( [ $T:ty, $test_fn:ident ], )* ) => {
            mod read_write_bytes {
                use super::*;

                $(
                    #[test]
                    fn $test_fn() {
                        use rand::{Rng, SeedableRng, rngs::StdRng};

                        let mut rng = StdRng::from_seed(SEED);

                        for _ in 0..TRIALS {
                            let val = rng.gen::<$T>();
                            let mut bytes = [0u8; std::mem::size_of::<$T>()];
                            unsafe {
                                val.write_bytes(&mut bytes);
                            }
                            assert_eq!(unsafe { <$T as Bytes>::read_bytes(&bytes) }, val);
                        }
                    }
                )*
            }
        };
    }

    // first time as a type, second time as an identifier
    test_bytes!(
        [i8, i8],
        [i16, i16],
        [i32, i32],
        [i64, i64],
        [i128, i128],
        [isize, isize],
        [u8, u8],
        [u16, u16],
        [u32, u32],
        [u64, u64],
        [u128, u128],
        [usize, usize],
        [f32, f32],
        [f64, f64],
        [char, char],
        [bool, bool],
        [(), unit],
    );

    mod pointer {
        use super::*;

        #[test]
        fn debug() {
            let mut builder = MemoryBuilder::new();
            let offset_0 = Pointer::new(&mut builder, 0_u64);
            let offset_8 = Pointer::new(&mut builder, false);

            assert_eq!(format!("{:?}", offset_0), "Pointer { offset: 0 }");
            assert_eq!(format!("{:?}", offset_8), "Pointer { offset: 8 }");
        }

        #[test]
        fn get_set_update() {
            let mut builder = MemoryBuilder::new();
            let pointer = Pointer::new(&mut builder, 5);
            let mut memory = builder.finish();

            assert_eq!(pointer.get(&memory), 5);

            pointer.set(&mut memory, 6);
            assert_eq!(pointer.get(&memory), 6);

            pointer.update(&mut memory, |x| x - 1);
            assert_eq!(pointer.get(&memory), 5);
        }
    }

    mod array_pointer {
        use super::*;

        #[test]
        fn debug() {
            let mut builder = MemoryBuilder::new();
            let offset_0 = ArrayPointer::new(&mut builder, &[0u64; 8]);
            let offset_64 = ArrayPointer::new(&mut builder, &[false]);

            assert_eq!(
                format!("{:?}", offset_0),
                "ArrayPointer { offset: 0, len: 8 }"
            );
            assert_eq!(
                format!("{:?}", offset_64),
                "ArrayPointer { offset: 64, len: 1 }"
            );
        }

        #[test]
        fn empty() {
            let mut builder = MemoryBuilder::new();
            let empty = ArrayPointer::<char>::new(&mut builder, &[]);
            let not_empty = ArrayPointer::new(&mut builder, &['a', 'b', 'c']);

            assert!(empty.is_empty());
            assert!(empty.len() == 0);

            assert!(!not_empty.is_empty());
            assert!(not_empty.len() != 0);
        }

        #[test]
        fn get_set_update() {
            let values = [1, 3, 5, 7];
            let mut builder = MemoryBuilder::new();
            let pointer = ArrayPointer::new(&mut builder, &values);
            let mut memory = builder.finish();

            for i in 0..4 {
                assert_eq!(pointer.get(&memory, i), values[i]);

                pointer.set(&mut memory, i, values[i] + 1);
                assert_eq!(pointer.get(&memory, i), values[i] + 1);

                pointer.update(&mut memory, i, |x| x - 1);
                assert_eq!(pointer.get(&memory, i), values[i]);
            }
        }

        #[test]
        fn swap() {
            let mut builder = MemoryBuilder::new();
            let pointer = ArrayPointer::new(&mut builder, &['a', 'z']);
            let mut memory = builder.finish();

            assert_eq!(pointer.get(&memory, 0), 'a');
            assert_eq!(pointer.get(&memory, 1), 'z');

            pointer.swap(&mut memory, 0, 1);

            assert_eq!(pointer.get(&memory, 0), 'z');
            assert_eq!(pointer.get(&memory, 1), 'a');
        }
    }
}
