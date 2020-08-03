extern crate proc_macro;

use proc_macro::TokenStream;

fn is_phantom_data_of(field: &syn::Field, type_param: &syn::Ident) -> bool {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { ref segments, .. },
        ..
    }) = field.ty
    {
        let syn::PathSegment { ident, arguments } = segments.last().unwrap();

        if ident != "PhantomData" {
            return false;
        }

        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args,
            ..
        }) = arguments
        {
            if let syn::GenericArgument::Type(syn::Type::Path(type_path)) =
                args.iter().next().unwrap()
            {
                let ty = type_path.path.segments.iter().next().unwrap();
                return ty.ident == *type_param;
            }
        }
    }

    false
}

fn associated_type_used(field: &syn::Field, type_param: &syn::Ident) -> bool {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { ref segments, .. },
        ..
    }) = field.ty
    {}
    false
}

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    // Setup
    let input: syn::DeriveInput = syn::parse(input).unwrap();

    // get the fields
    let type_name = input.ident;
    let fields = if let syn::Data::Struct(data_struct) = input.data {
        data_struct.fields
    } else {
        return syn::Error::new(type_name.span(), "CustomDebug only valid for structs.")
            .to_compile_error()
            .into();
    };

    // prepare the fields for quoting
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

    // handle generics
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let empty_where = syn::WhereClause {
        where_token: syn::Token![where](proc_macro2::Span::call_site()),
        predicates: syn::punctuated::Punctuated::new(),
    };

    let mut where_clause = where_clause.unwrap_or(&empty_where).to_owned();

    'outer: for param in input.generics.type_params() {
        let param_ident = &param.ident;

        for field in fields.iter() {
            if is_phantom_data_of(field, param_ident) {
                continue 'outer;
            }
        }
        where_clause
            .predicates
            .push_value(syn::parse_quote!( #param_ident: std::fmt::Debug ));
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
