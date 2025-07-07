use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod client;
mod parser;
mod server;

use parser::FileServiceDefinition;

#[proc_macro]
pub fn file_service(input: TokenStream) -> TokenStream {
    let definition = parse_macro_input!(input as FileServiceDefinition);

    let server_code = server::generate_server(&definition);
    let client_code = client::generate_client(&definition);

    let expanded = quote! {
        #server_code
        #client_code
    };

    TokenStream::from(expanded)
}
