use syn::{
    Error, Ident, LitStr, Result, Token, Type, braced,
    parse::{Parse, ParseStream},
    token,
};

#[derive(Debug)]
pub struct FileServiceDefinition {
    pub service_name: Ident,
    pub base_path: LitStr,
    pub body_limit: Option<u64>,
    pub openapi: Option<OpenApiConfig>,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug)]
pub enum OpenApiConfig {
    Enabled,
    WithPath(String),
}

#[derive(Debug)]
pub struct Endpoint {
    pub operation: Operation,
    pub auth: AuthRequirement,
    pub name: Ident,
    pub path: Option<LitStr>,
    pub path_params: Vec<PathParam>,
    pub response_type: Option<Type>,
}

#[derive(Debug)]
pub enum Operation {
    Upload,
    Download,
}

#[derive(Debug)]
pub enum AuthRequirement {
    Unauthorized,
    WithPermissions(Vec<Vec<String>>),
}

#[derive(Debug)]
pub struct PathParam {
    pub name: Ident,
    pub ty: Type,
}

impl Parse for FileServiceDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        braced!(content in input);

        let mut service_name = None;
        let mut base_path = None;
        let mut body_limit = None;
        let mut openapi = None;
        let mut endpoints = Vec::new();

        while !content.is_empty() {
            let field_name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;

            match field_name.to_string().as_str() {
                "service_name" => {
                    service_name = Some(content.parse()?);
                }
                "base_path" => {
                    base_path = Some(content.parse()?);
                }
                "body_limit" => {
                    let lit: syn::LitInt = content.parse()?;
                    body_limit = Some(lit.base10_parse()?);
                }
                "openapi" => {
                    // Parse openapi value - can be true/false or { output: "path" }
                    if content.peek(syn::LitBool) {
                        let enabled = content.parse::<syn::LitBool>()?;
                        if enabled.value() {
                            openapi = Some(OpenApiConfig::Enabled);
                        }
                    } else if content.peek(syn::token::Brace) {
                        let openapi_content;
                        syn::braced!(openapi_content in content);

                        // Parse output: "path"
                        let _ = openapi_content.parse::<Ident>()?; // "output"
                        openapi_content.parse::<Token![:]>()?;
                        let path = openapi_content.parse::<LitStr>()?;
                        openapi = Some(OpenApiConfig::WithPath(path.value()));
                    }
                }
                "endpoints" => {
                    let endpoints_content;
                    syn::bracketed!(endpoints_content in content);

                    while !endpoints_content.is_empty() {
                        endpoints.push(endpoints_content.parse()?);

                        if !endpoints_content.is_empty() {
                            endpoints_content.parse::<Token![,]>()?;
                        }
                    }
                }
                _ => {
                    return Err(Error::new(
                        field_name.span(),
                        format!("Unknown field: {}", field_name),
                    ));
                }
            }

            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(FileServiceDefinition {
            service_name: service_name
                .ok_or_else(|| Error::new(input.span(), "Missing service_name"))?,
            base_path: base_path.ok_or_else(|| Error::new(input.span(), "Missing base_path"))?,
            body_limit,
            openapi,
            endpoints,
        })
    }
}

impl Parse for Endpoint {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse operation type (UPLOAD or DOWNLOAD)
        let operation = if input.peek(kw::UPLOAD) {
            input.parse::<kw::UPLOAD>()?;
            Operation::Upload
        } else if input.peek(kw::DOWNLOAD) {
            input.parse::<kw::DOWNLOAD>()?;
            Operation::Download
        } else {
            return Err(Error::new(input.span(), "Expected UPLOAD or DOWNLOAD"));
        };

        // Parse auth requirement
        let auth = if input.peek(kw::UNAUTHORIZED) {
            input.parse::<kw::UNAUTHORIZED>()?;
            AuthRequirement::Unauthorized
        } else if input.peek(kw::WITH_PERMISSIONS) {
            input.parse::<kw::WITH_PERMISSIONS>()?;

            let content;
            syn::parenthesized!(content in input);

            let perms_content;
            syn::bracketed!(perms_content in content);

            let mut permission_groups = Vec::new();

            while !perms_content.is_empty() {
                if perms_content.peek(syn::LitStr) {
                    // Single permission
                    let perm: LitStr = perms_content.parse()?;
                    permission_groups.push(vec![perm.value()]);
                } else if perms_content.peek(token::Bracket) {
                    // Permission group
                    let group_content;
                    syn::bracketed!(group_content in perms_content);

                    let mut group = Vec::new();
                    while !group_content.is_empty() {
                        let perm: LitStr = group_content.parse()?;
                        group.push(perm.value());

                        if !group_content.is_empty() {
                            group_content.parse::<Token![,]>()?;
                        }
                    }
                    permission_groups.push(group);
                }

                if !perms_content.is_empty() {
                    perms_content.parse::<Token![,]>()?;
                }
            }

            AuthRequirement::WithPermissions(permission_groups)
        } else {
            return Err(Error::new(
                input.span(),
                "Expected UNAUTHORIZED or WITH_PERMISSIONS",
            ));
        };

        // Parse endpoint path and name
        let (name, path, path_params) = if input.peek(Ident) && input.peek2(Token![/]) {
            // Has custom path
            let mut segments = Vec::new();
            let mut params = Vec::new();

            // Parse path segments
            while input.peek(Ident) || input.peek(Token![/]) {
                if input.peek(Token![/]) {
                    input.parse::<Token![/]>()?;
                    segments.push("/".to_string());
                }

                if input.peek(Ident) {
                    let ident: Ident = input.parse()?;
                    segments.push(ident.to_string());
                } else if input.peek(token::Brace) {
                    // Path parameter
                    let content;
                    braced!(content in input);

                    let param_name: Ident = content.parse()?;
                    content.parse::<Token![:]>()?;
                    let param_type: Type = content.parse()?;

                    segments.push(format!("{{{}}}", param_name));
                    params.push(PathParam {
                        name: param_name,
                        ty: param_type,
                    });
                }
            }

            // Extract the method name from the path
            let method_name = segments
                .iter()
                .filter(|s| !s.starts_with('/') && !s.starts_with('{'))
                .cloned()
                .collect::<Vec<_>>()
                .join("_");

            let name = Ident::new(&method_name, input.span());
            let path = LitStr::new(&segments.join(""), input.span());

            (name, Some(path), params)
        } else {
            // Just method name
            let name: Ident = input.parse()?;
            (name, None, Vec::new())
        };

        // Parse parameters and response
        // For file operations, we expect empty parentheses
        let _content;
        syn::parenthesized!(_content in input);

        let response_type = if input.peek(Token![->]) {
            input.parse::<Token![->]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Endpoint {
            operation,
            auth,
            name,
            path,
            path_params,
            response_type,
        })
    }
}

mod kw {
    syn::custom_keyword!(UPLOAD);
    syn::custom_keyword!(DOWNLOAD);
    syn::custom_keyword!(UNAUTHORIZED);
    syn::custom_keyword!(WITH_PERMISSIONS);
}
