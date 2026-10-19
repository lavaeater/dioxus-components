//! Parsing and code generation for the `attributes!` macro

use dioxus_rsx::{Attribute, ElementName};
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    Expr, Ident, Token,
};

/// A parsed list of attributes from the `attributes!` macro
pub struct AttributeList {
    /// The element name for namespace/volatility lookup
    #[allow(dead_code)]
    pub element_name: ElementName,
    /// Regular attributes (not spreads)
    pub attributes: Vec<Attribute>,
    /// Spread expressions that provide existing collections of attributes
    pub spreads: Vec<SpreadExpr>,
}

/// A spread expression like `..existing_attrs`
pub struct SpreadExpr {
    #[allow(dead_code)]
    pub dots: Token![..],
    pub expr: Expr,
}

impl Parse for AttributeList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse element name first (e.g., `div`, `input`, `button`)
        let element_name: ElementName = if input.peek(Ident) {
            ElementName::Ident(input.parse()?)
        } else {
            return Err(input.error("expected element name (e.g., div, input, button)"));
        };

        // Parse the braced content
        let content;
        braced!(content in input);

        let mut attributes = Vec::new();
        let mut spreads = Vec::new();

        while !content.is_empty() {
            // Check for spread attribute first
            if content.peek(Token![..]) {
                let dots = content.parse::<Token![..]>()?;
                let expr = content.parse::<Expr>()?;
                // Consume optional trailing comma
                let _ = content.parse::<Token![,]>();
                spreads.push(SpreadExpr { dots, expr });
                continue;
            }

            // Parse regular attribute using dioxus_rsx's parser
            let mut attr: Attribute = content.parse()?;
            // Set the element name for proper namespace/volatility lookup
            attr.el_name = Some(element_name.clone());
            attributes.push(attr);
        }

        Ok(AttributeList {
            element_name,
            attributes,
            spreads,
        })
    }
}

impl ToTokens for AttributeList {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let attr_tokens: Vec<TokenStream2> = self
            .attributes
            .iter()
            .map(|attr| {
                let rendered = attr.rendered_as_dynamic_attr();
                quote! {
                    __attrs.extend(__IntoAttrs::__into_attrs(#rendered));
                }
            })
            .collect();

        let spread_tokens: Vec<TokenStream2> = self
            .spreads
            .iter()
            .map(|spread| {
                let expr = &spread.expr;
                quote! {
                    __attrs.extend(#expr);
                }
            })
            .collect();

        let output = quote! {
            {
                // dioxus-rsx renders an attribute as a `Box<[Attribute; N]>`,
                // a `Box<[Attribute]>` or a `Vec<Attribute>` depending on its
                // kind. A boxed array is not `IntoIterator`, so normalise them
                // all before extending.
                trait __IntoAttrs<T> {
                    fn __into_attrs(self) -> ::std::vec::Vec<T>;
                }
                impl<T> __IntoAttrs<T> for ::std::vec::Vec<T> {
                    fn __into_attrs(self) -> ::std::vec::Vec<T> {
                        self
                    }
                }
                impl<T> __IntoAttrs<T> for ::std::boxed::Box<[T]> {
                    fn __into_attrs(self) -> ::std::vec::Vec<T> {
                        self.into_vec()
                    }
                }
                impl<T, const N: usize> __IntoAttrs<T> for ::std::boxed::Box<[T; N]> {
                    fn __into_attrs(self) -> ::std::vec::Vec<T> {
                        let slice: ::std::boxed::Box<[T]> = self;
                        slice.into_vec()
                    }
                }
                let mut __attrs: Vec<_> = Vec::new();
                #( #attr_tokens )*
                #( #spread_tokens )*
                __attrs
            }
        };

        tokens.extend(output);
    }
}
