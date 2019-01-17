/*
 * This Source Code Form is subject to the terms of the Mozilla Public License,
 * v. 2.0. If a copy of the MPL was not distributed with this file, You can
 * obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Custom derive for `contrail::mem::Bytes`.
//!
//! This crate is internal to `contrail`; there's no reason to import it yourself.
#![recursion_limit = "128"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate syn;

#[proc_macro_derive(Bytes)]
pub fn bytes_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let derive_input: syn::DeriveInput =
        syn::parse2(input).expect("could not parse as syn::DeriveInput");
    let name = &derive_input.ident;

    if derive_input.generics.type_params().count() > 0
        || derive_input.generics.lifetimes().count() > 0
        || derive_input.generics.const_params().count() > 0
    {
        panic!("cannot derive Bytes for structs with generic parameters");
    }

    let impl_tokens: proc_macro2::TokenStream = quote! {
        impl contrail::mem::Bytes for #name {
            const LENGTH: usize = std::mem::size_of::<#name>();

            #[inline(always)]
            unsafe fn read_bytes(bytes: &[u8]) -> #name {
                // safe assuming that the length of the byte slice is Self::LENGTH.
                let byte_array = *(bytes.as_ptr() as *const [u8; std::mem::size_of::<#name>()]);
                // safe assuming that the byte slice represents a valid value of type T.
                std::mem::transmute::<[u8; std::mem::size_of::<#name>()], #name>(byte_array)
            }

            #[inline(always)]
            unsafe fn write_bytes(self, bytes: &mut [u8]) {
                // safe for Copy + 'static types
                let byte_array = std::mem::transmute::<#name, [u8; std::mem::size_of::<#name>()]>(self);
                // safe assuming that the length of the byte slice is Self::LENGTH.
                bytes.copy_from_slice(&byte_array);
            }
        }
    };

    impl_tokens.into()
}
