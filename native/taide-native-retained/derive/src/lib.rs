use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, parse_macro_input, parse_quote};

#[proc_macro_derive(RetainedBytes)]
pub fn derive_retained_bytes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<Tokens> {
    let name = &input.ident;
    let mut generics = input.generics.clone();
    for parameter in generics.type_params_mut() {
        parameter
            .bounds
            .push(parse_quote!(::taide_native_retained::RetainedBytes));
    }
    let (implementation, parameters, constraints) = generics.split_for_impl();
    let mut types = Vec::new();
    let body = match &input.data {
        Data::Struct(data) => {
            let members = data.fields.members();
            types.extend(data.fields.iter().map(|field| &field.ty));
            quote! { #(__taide_retained_visitor.push(&self.#members)?;)* }
        }
        Data::Enum(data) => {
            let arms = data
                .variants
                .iter()
                .map(|variant| {
                    let variant_name = &variant.ident;
                    let names: Vec<_> = (0..variant.fields.len())
                        .map(|index| format_ident!("__taide_retained_field_{index}"))
                        .collect();
                    types.extend(variant.fields.iter().map(|field| &field.ty));
                    let pattern = match &variant.fields {
                        Fields::Named(fields) => {
                            let members = fields.named.iter().map(|field| &field.ident);
                            quote! { Self::#variant_name { #(#members: #names),* } }
                        }
                        Fields::Unnamed(_) => quote! { Self::#variant_name(#(#names),*) },
                        Fields::Unit => quote! { Self::#variant_name },
                    };
                    quote! { #pattern => { #(__taide_retained_visitor.push(#names)?;)* } }
                })
                .collect::<Vec<_>>();
            quote! { match self { #(#arms),* } }
        }
        Data::Union(data) => {
            return Err(syn::Error::new_spanned(
                data.union_token,
                "RetainedBytes does not support unions",
            ));
        }
    };
    Ok(quote! {
        #[automatically_derived]
        impl #implementation ::taide_native_retained::RetainedBytes for #name #parameters #constraints {
            fn has_owned_children() -> bool {
                false #(|| <#types as ::taide_native_retained::RetainedBytes>::has_owned_children())*
            }
            fn visit<'__taide_retained_value>(
                &'__taide_retained_value self,
                __taide_retained_visitor: &mut ::taide_native_retained::Visitor<'__taide_retained_value, '_>,
            ) -> Result<(), ::taide_native_retained::Error> {
                #body
                Ok(())
            }
        }
    })
}
