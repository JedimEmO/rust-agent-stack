use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, LitStr, Token, Type, parse::Parse, parse_macro_input};

mod client;
mod server;

#[cfg(test)]
mod tests;

/// Macro to generate a bidirectional JSON-RPC service with client and server code
///
/// This macro generates:
/// - Server trait with methods for client_to_server handlers
/// - Server builder with WebSocket integration  
/// - Client struct with type-safe method calls and notification handlers
/// - Type-safe message enums for both directions
///
/// See the tests for usage examples.
#[proc_macro]
pub fn jsonrpc_bidirectional_service(input: TokenStream) -> TokenStream {
    let service_definition = parse_macro_input!(input as BidirectionalServiceDefinition);

    match generate_service_code(service_definition) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[derive(Debug)]
struct BidirectionalServiceDefinition {
    service_name: Ident,
    client_to_server: Vec<MethodDefinition>,
    server_to_client: Vec<NotificationDefinition>,
}

#[derive(Debug)]
struct MethodDefinition {
    auth: AuthRequirement,
    name: Ident,
    request_type: Type,
    response_type: Type,
}

#[derive(Debug)]
struct NotificationDefinition {
    name: Ident,
    params_type: Type,
}

#[derive(Debug)]
enum AuthRequirement {
    Unauthorized,
    WithPermissions(Vec<Vec<String>>), // Vec of permission groups - OR between groups, AND within groups
}

impl Parse for BidirectionalServiceDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse the opening brace
        let content;
        syn::braced!(content in input);

        // Parse service_name: Ident
        let _ = content.parse::<Ident>()?; // "service_name"
        let _ = content.parse::<Token![:]>()?;
        let service_name = content.parse::<Ident>()?;
        let _ = content.parse::<Token![,]>()?;

        // Parse client_to_server: [...]
        let _ = content.parse::<Ident>()?; // "client_to_server"
        let _ = content.parse::<Token![:]>()?;

        let client_to_server_content;
        syn::bracketed!(client_to_server_content in content);

        let mut client_to_server = Vec::new();
        while !client_to_server_content.is_empty() {
            let method = client_to_server_content.parse::<MethodDefinition>()?;
            client_to_server.push(method);

            // Handle optional trailing comma
            if client_to_server_content.peek(Token![,]) {
                let _ = client_to_server_content.parse::<Token![,]>()?;
            }
        }

        let _ = content.parse::<Token![,]>()?;

        // Parse server_to_client: [...]
        let _ = content.parse::<Ident>()?; // "server_to_client"
        let _ = content.parse::<Token![:]>()?;

        let server_to_client_content;
        syn::bracketed!(server_to_client_content in content);

        let mut server_to_client = Vec::new();
        while !server_to_client_content.is_empty() {
            let notification = server_to_client_content.parse::<NotificationDefinition>()?;
            server_to_client.push(notification);

            // Handle optional trailing comma
            if server_to_client_content.peek(Token![,]) {
                let _ = server_to_client_content.parse::<Token![,]>()?;
            }
        }

        Ok(BidirectionalServiceDefinition {
            service_name,
            client_to_server,
            server_to_client,
        })
    }
}

impl Parse for MethodDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse auth requirement (UNAUTHORIZED or WITH_PERMISSIONS([...]))
        let auth = if input.peek(syn::Ident) {
            let auth_ident = input.parse::<Ident>()?;
            match auth_ident.to_string().as_str() {
                "UNAUTHORIZED" => AuthRequirement::Unauthorized,
                "WITH_PERMISSIONS" => {
                    // Parse ([...] | [...] | ...)
                    let perms_content;
                    syn::parenthesized!(perms_content in input);

                    let mut permission_groups = Vec::new();

                    // Parse first permission group
                    let first_group_content;
                    syn::bracketed!(first_group_content in perms_content);

                    let mut first_group = Vec::new();
                    while !first_group_content.is_empty() {
                        let perm = first_group_content.parse::<LitStr>()?;
                        first_group.push(perm.value());

                        if first_group_content.peek(Token![,]) {
                            let _ = first_group_content.parse::<Token![,]>()?;
                        }
                    }
                    permission_groups.push(first_group);

                    // Parse additional permission groups separated by |
                    while perms_content.peek(Token![|]) {
                        let _ = perms_content.parse::<Token![|]>()?;

                        let group_content;
                        syn::bracketed!(group_content in perms_content);

                        let mut group = Vec::new();
                        while !group_content.is_empty() {
                            let perm = group_content.parse::<LitStr>()?;
                            group.push(perm.value());

                            if group_content.peek(Token![,]) {
                                let _ = group_content.parse::<Token![,]>()?;
                            }
                        }
                        permission_groups.push(group);
                    }

                    AuthRequirement::WithPermissions(permission_groups)
                }
                _ => {
                    return Err(syn::Error::new(
                        auth_ident.span(),
                        "Expected UNAUTHORIZED or WITH_PERMISSIONS",
                    ));
                }
            }
        } else {
            return Err(syn::Error::new(
                input.span(),
                "Expected authentication requirement",
            ));
        };

        // Parse method name
        let name = input.parse::<Ident>()?;

        // Parse (RequestType)
        let request_content;
        syn::parenthesized!(request_content in input);
        let request_type = request_content.parse::<Type>()?;

        // Parse -> ResponseType
        let _ = input.parse::<Token![->]>()?;
        let response_type = input.parse::<Type>()?;

        Ok(MethodDefinition {
            auth,
            name,
            request_type,
            response_type,
        })
    }
}

impl Parse for NotificationDefinition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse notification name
        let name = input.parse::<Ident>()?;

        // Parse (ParamsType)
        let params_content;
        syn::parenthesized!(params_content in input);
        let params_type = params_content.parse::<Type>()?;

        Ok(NotificationDefinition { name, params_type })
    }
}

fn generate_service_code(
    service_def: BidirectionalServiceDefinition,
) -> syn::Result<proc_macro2::TokenStream> {
    // Generate server code - this will be conditionally compiled by the user
    let server_code = server::generate_server_code(&service_def);

    // Generate client code - this will be conditionally compiled by the user
    let client_code = client::generate_client_code(&service_def);

    let output = quote! {
        #server_code
        #client_code
    };

    Ok(output)
}
