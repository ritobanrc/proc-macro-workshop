extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();

    let type_name = input.ident;
    let fields = if let syn::Data::Struct(data_struct) = input.data {
        data_struct.fields
    } else {
        return syn::Error::new(type_name.span(), "CustomDebug only valid for structs.")
            .to_compile_error()
            .into();
    };

    let field_names = fields.iter().filter_map(|f| f.ident.as_ref());
    let field_values = fields.iter().filter_map(|f| {
        let field_name = f.ident.as_ref()?;
        let format_str = f
            .attrs
            .iter()
            .filter_map(|attr| {
                if let syn::Meta::NameValue(nv) = attr.parse_meta().ok()? {
                    if nv.path.is_ident("debug") {
                        if let syn::Lit::Str(s) = nv.lit {
                            return Some(s.value());
                        }
                    }
                }
                None
            })
            .next();
        if let Some(format_str) = format_str {
            Some(quote::quote! {
                &format_args!(#format_str, &self.#field_name)
            })
        } else {
            Some(quote::quote! { &self.#field_name })
        }
    });

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let empty_where = syn::WhereClause {
        where_token: syn::Token![where](proc_macro2::Span::call_site()),
        predicates: syn::punctuated::Punctuated::new(),
    };

    let mut where_clause = where_clause.unwrap_or(&empty_where).to_owned();

    for param in input.generics.type_params() {
        let ident = &param.ident;
        where_clause
            .predicates
            .push_value(syn::parse_quote!( #ident: std::fmt::Debug ))
    }

    (quote::quote! {
        impl #impl_generics std::fmt::Debug for #type_name #ty_generics #where_clause {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct(stringify!(#type_name))
                    #( .field(stringify!(#field_names), #field_values) )*
                    .finish()
            }
        }
    })
    .into()
}
