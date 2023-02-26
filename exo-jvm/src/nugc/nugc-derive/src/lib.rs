use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{Data, Type, Member};



/// Generates setter methods that maintain the 
/// write barrier. Do not write the fields directly!
#[proc_macro_attribute]
pub fn generate_write_barriers(f: TokenStream, a: TokenStream) -> TokenStream {
    let v: syn::DeriveInput = syn::parse(a).unwrap();

    match &v.data {
        Data::Struct(s) => {
            
            let mut fields = vec![];

            'm: for field in &s.fields {
                match &field.ty {
                    Type::Path(v) => {
                        if v.path.segments.last().unwrap().ident.to_string() == "GcPtr" {
                            fields.push(field);
                        }
                    }
                    v => panic!("{:?}", std::mem::discriminant(v))
                }
            }

            let mut types: Vec<Type> = fields.iter().map(|v| v.ty.clone()).collect();

            let mut n = 0;
            let mut paths: Vec<syn::Member> = fields.iter().map(|v| v.ident.clone().map(|v| syn::Member::Named(v)).unwrap_or_else(|| {n+=1; syn::Member::Unnamed(syn::Index { index: n-1, span: Span::call_site() })})).collect();


            let strings: Vec<_> = paths.iter().map(|v| match v {
                Member::Named(v) => v.to_string(),
                Member::Unnamed(v) => v.index.to_string()
            }).collect();

            let setters: Vec<_> = strings.iter().map(|v| quote::format_ident!("set_{}", v)).collect();

            let i = v.ident.clone();

            quote! {
                #v
                
                impl #i {
                    #(
                        pub fn #setters(&mut self, our_ref: GcPtr<Self>, v: &GarbageCollector, new_v: #types) {
                            self.#paths = new_v;
                        }
                    )*
                }
            }.into()

        }
        _ => todo!()
    }
}




/// Generates a `Trace` implementation for the struct.
#[proc_macro_derive(Trace, attributes(unsafe_ignore_trace))]
pub fn object_impl(f: TokenStream) -> TokenStream {
    let v: syn::DeriveInput = syn::parse(f).unwrap();

    match v.data {
        Data::Struct(s) => {
            
            let mut fields = vec![];

            'm: for field in s.fields {
                match field.ty {
                    Type::Path(_) | Type::Tuple(_) => {
                        for attr in &field.attrs {
                            if attr.path.segments.last().unwrap().ident.to_string() == "unsafe_ignore_trace" {
                                continue 'm;
                            }
                        }
                        fields.push(field);
                    }
                    v => panic!("{:?}", std::mem::discriminant(&v))
                }
            }

            let mut types: Vec<Type> = fields.iter().map(|v| v.ty.clone()).collect();

            let mut n = 0;
            let mut paths: Vec<syn::Member> = fields.iter().map(|v| v.ident.clone().map(|v| syn::Member::Named(v)).unwrap_or_else(|| {n+=1; syn::Member::Unnamed(syn::Index { index: n-1, span: Span::call_site() })})).collect();

            let i = v.ident;
            let needs_traced = syn::LitBool {
                value: fields.len() > 0,
                span: Span::call_site()
            };
            quote! {
        
                unsafe impl Trace for #i {
                    const NEEDS_TRACED: bool = #(<#types>::NEEDS_TRACED) &&* && #needs_traced;
                    fn trace<V: Visitor>(
                        &mut self,
                        gc: &GarbageCollector,
                        visitor: &mut V,
                    ) {
                        #(
                            visitor.visit_noref(gc, &mut self.#paths);
                        )*
                    }
                }
        
                const _: fn() = || {
                    fn check_impl<T: ?Sized + Trace>() {}
                    #(
                        check_impl::<#types>();
                    )*
                };
        
            }.into()
        }
        _ => todo!()
    }
}