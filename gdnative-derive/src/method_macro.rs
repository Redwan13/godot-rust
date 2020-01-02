use syn::{Attribute, FnArg, ImplItem, ItemImpl, Pat, PatIdent, Signature, Type};

use gdnative_core::init::RpcMode;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Punct, Spacing, Span, TokenStream as TokStream2};
use quote::ToTokens;
use quote::TokenStreamExt;
use std::boxed::Box;
use std::str::FromStr;

pub(crate) struct MethodMetadata {
    pub(crate) signature: Signature,
    pub(crate) rpc_mode: RpcMode,
}

pub(crate) struct ClassMethodExport {
    pub(crate) class_ty: Box<Type>,
    pub(crate) methods: Vec<MethodMetadata>,
}

pub struct RpcModeWrapper {
    pub rpc_mode: RpcMode,
}

fn append_separator(tokens: &mut TokStream2) {
    tokens.append(Punct::new(':', Spacing::Joint));
    tokens.append(Punct::new(':', Spacing::Alone));
}

impl ToTokens for RpcModeWrapper {
    fn to_tokens(&self, tokens: &mut TokStream2) {
        tokens.append(Ident::new("gdnative", Span::call_site()));
        append_separator(tokens);
        tokens.append(Ident::new("init", Span::call_site()));
        append_separator(tokens);
        tokens.append(Ident::new("RpcMode", Span::call_site()));
        append_separator(tokens);
        tokens.append(Ident::new(&self.rpc_mode.to_string(), Span::call_site()));
    }
}

/// Parse the input.
///
/// Returns the TokenStream of the impl block together with a description of methods to export.
pub(crate) fn parse_method_export(
    _meta: TokenStream,
    input: TokenStream,
) -> (ItemImpl, ClassMethodExport) {
    let ast = match syn::parse_macro_input::parse::<ItemImpl>(input) {
        Ok(impl_block) => impl_block,
        Err(err) => {
            // if the impl block is ill-formed there is no point in error handling.
            panic!("{}", err);
        }
    };

    impl_gdnative_expose(ast)
}

fn find_attribute_position(attrs: &Vec<Attribute>, attr_name: &str) -> Option<usize> {
    let attribute_pos = attrs.iter().position(|attr| {
        let correct_style = match attr.style {
            syn::AttrStyle::Outer => true,
            _ => false,
        };

        for path in attr.path.segments.iter() {
            if path.ident.to_string() == attr_name {
                return correct_style;
            }
        }

        false
    });
    return attribute_pos;
}

fn parse_rpc_mode(attr: &Attribute) -> RpcMode {
    let rpc_type = attr
        .parse_args::<Type>()
        .expect("`rpc` attribute requires the RpcMode value as an argument.");
    let pth = match rpc_type {
        Type::Path(pth) => {
            let mut s = TokStream2::new();
            pth.path.to_tokens(&mut s);
            println!("+++ '{}'", s.to_string());
            pth.path
        }
        _ => panic!("`rpc` attribute requires the RpcMode value as an argument."),
    };

    let mut found = false;
    for seg in pth.segments.iter() {
        let val = seg.ident.to_string();
        if !found {
            found = val == "RpcMode";
        } else {
            return RpcMode::from_str(val.as_str()).unwrap();
        }
    }

    panic!("`rpc` attribute requires the RpcMode value as an argument.")
}

/// Extract the data to export from the impl block.
fn impl_gdnative_expose(ast: ItemImpl) -> (ItemImpl, ClassMethodExport) {
    // the ast input is used for inspecting.
    // this clone is used to remove all attributes so that the resulting
    // impl block actually compiles again.
    let mut result = ast.clone();

    // This is done by removing all items first, they will be added back on later
    result.items.clear();

    // data used for generating the exported methods.
    let mut export = ClassMethodExport {
        class_ty: ast.self_ty,
        methods: vec![],
    };

    let mut methods_to_export = Vec::<MethodMetadata>::new();

    // extract all methods that have the #[export] attribute.
    // add all items back to the impl block again.
    for func in ast.items {
        let item = match func {
            ImplItem::Method(mut method) => {
                // only allow the "outer" style, aka #[thing] item.
                let export_attr_pos = find_attribute_position(&method.attrs, "export");

                if let Some(idx) = export_attr_pos {
                    let rpc_attr_pos = find_attribute_position(&method.attrs, "rpc");
                    let rpc_mode = match rpc_attr_pos {
                        None => RpcMode::Disabled,
                        Some(pos) => {
                            let attr = method.attrs.remove(pos);
                            let rpc_mode = parse_rpc_mode(&attr);
                            rpc_mode
                        }
                    };

                    // TODO renaming?
                    let _attr = method.attrs.remove(idx);

                    let meta = MethodMetadata {
                        signature: method.sig.clone(),
                        rpc_mode: rpc_mode,
                    };
                    methods_to_export.push(meta);
                }

                ImplItem::Method(method)
            }
            item => item,
        };

        result.items.push(item);
    }

    // check if the export methods have the proper "shape", the write them
    // into the list of things to export.
    {
        for mut method in methods_to_export {
            let generics = &method.signature.generics;

            if generics.type_params().count() > 0 {
                eprintln!("type parameters not allowed in exported functions");
                continue;
            }
            if generics.lifetimes().count() > 0 {
                eprintln!("lifetime parameters not allowed in exported functions");
                continue;
            }
            if generics.const_params().count() > 0 {
                eprintln!("const parameters not allowed in exported functions");
                continue;
            }

            // remove "mut" from arguments.
            // give every wildcard a (hopefully) unique name.
            method
                .signature
                .inputs
                .iter_mut()
                .enumerate()
                .for_each(|(i, arg)| match arg {
                    FnArg::Typed(cap) => match *cap.pat.clone() {
                        Pat::Wild(_) => {
                            let name = format!("___unused_arg_{}", i);

                            cap.pat = Box::new(Pat::Ident(PatIdent {
                                attrs: vec![],
                                by_ref: None,
                                mutability: None,
                                ident: syn::Ident::new(&name, Span::call_site()),
                                subpat: None,
                            }));
                        }
                        Pat::Ident(mut ident) => {
                            ident.mutability = None;
                            cap.pat = Box::new(Pat::Ident(ident));
                        }
                        _ => {}
                    },
                    _ => {}
                });

            // The calling site is already in an unsafe block, so removing it from just the
            // exported binding is fine.
            method.signature.unsafety = None;

            export.methods.push(method);
        }
    }

    (result, export)
}
