use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn file_service_debug(input: TokenStream) -> TokenStream {
    // Just echo back what we received to see if it's being tokenized correctly
    eprintln!("Raw input tokens: {:?}", input);
    
    // Try to convert to a TokenStream2 and print
    let tokens2 = proc_macro2::TokenStream::from(input.clone());
    eprintln!("TokenStream2: {:?}", tokens2);
    
    // Return empty expansion
    quote! {
        // Debug macro - no real output
    }.into()
}