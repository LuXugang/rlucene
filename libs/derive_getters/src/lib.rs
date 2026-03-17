/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Type, parse_macro_input};

#[proc_macro_derive(Getters)]
pub fn derive_getters(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let struct_name = &input.ident;

  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let fields = match &input.data {
    syn::Data::Struct(data) => match &data.fields {
      syn::Fields::Named(fields_named) => &fields_named.named,
      _ => unimplemented!("Only named fields are supported by #[derive(Getters)]"),
    },
    _ => unimplemented!("Only structs are supported by #[derive(Getters)]"),
  };

  let getters = fields.iter().map(|field| {
    let field_name = field.ident.as_ref().unwrap();
    let field_type = &field.ty;
    let getter_name = format_ident!("get_{}", field_name);

    if is_copy_type(field_type) {
      quote! {
          pub fn #getter_name(&self) -> #field_type {
              self.#field_name
          }
      }
    } else {
      quote! {
          pub fn #getter_name(&self) -> &#field_type {
              &self.#field_name
          }
      }
    }
  });
  let expanded = quote! {
      impl #impl_generics #struct_name #ty_generics #where_clause {
          #(#getters)*
      }
  };

  TokenStream::from(expanded)
}

fn is_copy_type(ty: &Type) -> bool {
  match ty {
    Type::Path(type_path) => {
      if let Some(segment) = type_path.path.segments.last() {
        matches!(
          segment.ident.to_string().as_str(),
          "bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "isize"
            | "f32"
            | "f64"
            | "char"
        )
      } else {
        false
      }
    },
    _ => false,
  }
}
