#![expect(clippy::unreachable, reason = "generated code from darling::FromAttributes")]

//! Procedural macro implementation details for `nkcore-debug`.

use proc_macro2::*;
use quote::quote;
use syn::*;
use syn::parse::*;
use tap::prelude::*;
use darling::*;

/// Parsed attributes and expression supplied to [`api_name_of_internal`].
struct ApiNameOfInput {
    /// Configuration injected by the public declarative macro.
    attributes: Vec<Attribute>,
    /// Function or method call whose readable name should be generated.
    expr: Expr,
}

impl Parse for ApiNameOfInput {
    /// Parses configuration attributes followed by the target expression.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            attributes: input.call(Attribute::parse_outer)?,
            expr: input.parse()?,
        })
    }
}

/// Paths to runtime helpers supplied by the public declarative macro.
#[derive(FromAttributes)]
#[darling(attributes(api_name_args))]
struct ApiNameOfArgs {
    /// Produces a readable name for an explicitly known type.
    type_name: Path,
    /// Produces a readable name for a type inferred by an uncalled closure.
    receiver_type_name: Path,
    /// Constrains two references to have the same referent type.
    same_type: Path,
}

#[proc_macro]
/// Extracts a readable API name from a function or method call expression.
///
/// This implementation macro expects an `api_name_args` attribute followed by
/// a call expression. It supports type, lifetime, and explicitly supplied type
/// generic arguments; unsupported expressions and const generic arguments
/// produce a compile-time error.
pub fn api_name_of_internal(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ApiNameOfInput { attributes, expr } =
        parse_macro_input!(input as ApiNameOfInput);
    let args =
        match ApiNameOfArgs::from_attributes(&attributes) {
            Ok(args) => args,
            Err(error) => return error.write_errors().into(),
        };

    expand_api_name_of(args, expr)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Generates an expression that formats the target call's readable name.
///
/// # Errors
///
/// Returns an error at the unsupported expression or generic argument.
fn expand_api_name_of(args: ApiNameOfArgs, expr: Expr) -> syn::Result<TokenStream> {
    let ApiNameOfArgs {
        type_name,
        receiver_type_name,
        same_type,
    } = args;

    let mut expr = expr;
    loop {
        expr = match expr {
            Expr::Group(ExprGroup { expr, .. }) |
            Expr::Paren(ExprParen { expr, .. }) => *expr,
            _ => break,
        };
    };

    Ok(match expr {
        Expr::MethodCall(ExprMethodCall {
            ref receiver,
            ref method,
            ref turbofish,
            ..
        }) => {
            let method_ident =
                method.to_string();
            let method_generics =
                turbofish
                    .as_ref()
                    .map_or_else(
                        || Ok(quote!("")),
                        |turbofish| display_generic_args(turbofish, &type_name))?;
            quote!(format!(
                "{}::{}{}",
                #receiver_type_name(|receiver_type| {
                    #same_type(receiver_type, &(#receiver));
                }).trim_start_matches('&'),
                #method_ident,
                #method_generics))
        },
        Expr::Call(ExprCall { ref func, .. }) =>
            match func.as_ref() {
                &Expr::Path(ExprPath { path: Path { leading_colon, ref segments }, .. }) => {
                    let method =
                        segments
                            .last()
                            .ok_or_else(|| syn::Error::new_spanned(
                                func,
                                "expected a path to a free or associated function"))?;
                    let method_ident =
                        method.ident.to_string();
                    let method_generics =
                        if let PathArguments::AngleBracketed(ref args) = method.arguments {
                            display_generic_args(args, &type_name)?
                        } else {
                            quote!("")
                        };
                    if let Some(segment) =
                        segments.iter().nth_back(1) &&
                        segment
                            .ident
                            .to_string()
                            .chars()
                            .next()
                            .unwrap_or(' ')
                            .is_ascii_uppercase() {
                        let type_path = Path {
                            leading_colon,
                            segments:
                                segments
                                    .iter()
                                    .take(segments.len() - 1)
                                    .cloned()
                                    .collect(),
                        };
                        quote!(format!(
                            "{}::{}{}",
                            #type_name::<#type_path>(),
                            #method_ident,
                            #method_generics))
                    } else {
                        quote!(format!(
                            "{}{}",
                            #method_ident,
                            #method_generics))
                    }
                },
                _ => return Err(syn::Error::new_spanned(
                    func,
                    "expected a path to a free or associated function")),
            },
        _ => return Err(syn::Error::new_spanned(
            expr,
            "expected a function or method call")),
    })
}

/// Generates formatting code for explicitly supplied generic arguments.
///
/// # Errors
///
/// Returns an error for const generics and associated generic arguments.
fn display_generic_args(
    args: &AngleBracketedGenericArguments,
    type_name: &Path) -> syn::Result<TokenStream> {
    let args =
        args.args
        .iter()
        .map(|arg| match arg {
            &GenericArgument::Lifetime(ref lifetime) => {
                let lifetime = lifetime.to_string();
                Ok(quote!(#lifetime.to_string()))
            },
            &GenericArgument::Type(ref ty) => {
                Ok(quote!(#type_name::<#ty>()))
            },
            &GenericArgument::Const(ref expr) => {
                Err(syn::Error::new_spanned(
                    expr,
                    "const generic arguments are not supported"))
            },
            _ => {
                Err(syn::Error::new_spanned(
                    arg,
                    "associated generic arguments are not supported"))
            },
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(args.pipe(|args| quote!(format!("::<{}>", [#(#args),*].join(", ")))))
}
