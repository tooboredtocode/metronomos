use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(PulseValue)]
pub fn derive_pulse_value(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let expanded = quote! {
        impl #impl_generics metronomos_pulse::value::CustomPulseValue for #name #ty_generics #where_clause {
            const NAME: &'static str = stringify!(#name);
        }
    };

    // Hand the output tokens back to the compiler.
    TokenStream::from(expanded)
}
