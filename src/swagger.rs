use rocket_okapi::swagger_ui::SwaggerUIConfig;

pub fn swagger_ui() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "/api/openapi.json".to_string(),
        ..Default::default()
    }
}
