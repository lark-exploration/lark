extern crate proc_macro;
extern crate proc_macro2;

#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;

#[proc_macro_derive(DebugWith)]
pub fn derive_debug_with(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = &input.ident;

    // Generate an expression to sum up the heap size of each field.
    let debug_with = debug_with(&input);

    // Add a bound `T: HeapSize` to every type parameter T.
    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let syn::Generics {
        lt_token: _,
        params: impl_params,
        gt_token: _,
        where_clause: _,
    } = syn::parse_quote! { #impl_generics };

    let expanded = quote! {
        // The generated impl.
        impl < #(#impl_params,)* Cx > ::debug::DebugWith<Cx> for #name #ty_generics #where_clause {
            fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                #debug_with
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

// Add a bound `T: HeapSize` to every type parameter T.
fn add_trait_bounds(mut generics: syn::Generics) -> syn::Generics {
    // For each existing parameter, add `T: DebugWith`
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(syn::parse_quote!(::debug::DebugWith));
        }
    }

    generics
}

// Generate an expression to sum up the heap size of each field.
fn debug_with(input: &syn::DeriveInput) -> TokenStream {
    match &input.data {
        syn::Data::Struct(data) => debug_with_struct(&input.ident, data),
        syn::Data::Enum(data) => debug_with_variants(&input.ident, data),
        syn::Data::Union(_) => unimplemented!(),
    }
}

fn debug_with_variants(type_name: &syn::Ident, data: &syn::DataEnum) -> TokenStream {
    let variant_streams: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            match &variant.fields {
                syn::Fields::Named(fields) => {
                    let fnames: &Vec<_> = &fields.named.iter().map(|f| &f.ident).collect();
                    let fnames1: &Vec<_> = fnames;
                    quote! {
                        #type_name :: #variant_name { #(#fnames),* } => {
                            fmt.debug_struct(stringify!(#variant_name))
                                #(
                                    .field(stringify!(#fnames), &#fnames1.debug_with(cx))
                                )*
                            .finish()
                        }
                    }
                }

                syn::Fields::Unnamed(fields) => {
                    let all_names = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i"];
                    if fields.unnamed.len() > all_names.len() {
                        unimplemented!("too many variants")
                    }
                    let names = &all_names[0..fields.unnamed.len()];
                    quote! {
                        #type_name :: #variant_name { #(#names),* } => {
                            fmt.debug_tuple(stringify!(#variant_name))
                                #(
                                    .field(&#names.debug_with(cx))
                                )*
                            .finish()
                        }
                    }
                }

                syn::Fields::Unit => {
                    quote! {
                        #type_name :: #variant_name => {
                            fmt.debug_struct(stringify!(#variant_name)).finish()
                        }
                    }
                }
            }
        })
        .collect();

    quote! {
        match self {
            #(#variant_streams)*
        }
    }
}

fn debug_with_struct(type_name: &syn::Ident, data: &syn::DataStruct) -> TokenStream {
    match &data.fields {
        syn::Fields::Named(fields) => {
            // Expands to an expression like
            //
            //     fmt.debug_struct("foo").field("a", self.a.debug_with(cx)).finish()
            let fnames: &Vec<_> = &fields.named.iter().map(|f| &f.ident).collect();
            let fnames1 = fnames;
            quote! {
                fmt.debug_struct(stringify!(#type_name))
                #(
                    .field(stringify!(#fnames), &self.#fnames1.debug_with(cx))
                )*
                .finish()
            }
        }
        syn::Fields::Unnamed(fields) => {
            // Expands to an expression like
            //
            //     fmt.debug_tuple("foo").field(self.0.debug_with(cx)).finish()
            let indices = 0..fields.unnamed.len();
            quote! {
                fmt.debug_tuple(stringify!(#type_name))
                #(
                    .field(&self.#indices.debug_with(cx))
                )*
                .finish()
            }
        }
        syn::Fields::Unit => {
            quote! {
                fmt.debug_struct(stringify!(#type_name))
                .finish()
            }
        }
    }
}
