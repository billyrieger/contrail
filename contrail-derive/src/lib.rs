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

    if derive_input.generics.type_params().next().is_some()
        || derive_input.generics.lifetimes().next().is_some()
        || derive_input.generics.const_params().next().is_some()
    {
        panic!("cannot derive Bytes for structs with generic parameters");
    }

    let impl_tokens: proc_macro2::TokenStream = quote! {
        impl ::contrail::mem::Bytes for #name {
            const LENGTH: usize = std::mem::size_of::<#name>();

            #[inline(always)]
            unsafe fn read_bytes(bytes: &[u8]) -> #name {
                let byte_array = *(bytes.as_ptr() as *const [u8; std::mem::size_of::<#name>()]);
                std::mem::transmute::<[u8; std::mem::size_of::<#name>()], #name>(byte_array)
            }

            #[inline(always)]
            unsafe fn write_bytes(self, bytes: &mut [u8]) {
                let byte_array = std::mem::transmute::<#name, [u8; std::mem::size_of::<#name>()]>(self);
                bytes.copy_from_slice(&byte_array);
            }
        }
    };

    impl_tokens.into()
}
