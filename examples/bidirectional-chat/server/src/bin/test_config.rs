//! Test configuration loading

use bidirectional_chat_server::config::Config;

fn main() {
    println!("Testing configuration loading...\n");

    match Config::load() {
        Ok(config) => {
            println!("✓ Configuration loaded successfully!");
            println!("\nServer Configuration:");
            println!("  Host: {}", config.server.host);
            println!("  Port: {}", config.server.port);
            println!("  CORS: {:?}", config.server.cors.allow_any_origin);

            println!("\nAuth Configuration:");
            println!("  JWT TTL: {} seconds", config.auth.jwt_ttl_seconds);
            println!("  JWT Algorithm: {}", config.auth.jwt_algorithm);
            println!("  Refresh Enabled: {}", config.auth.refresh_enabled);

            println!("\nChat Configuration:");
            println!("  Data Directory: {:?}", config.chat.data_dir);
            println!("  Max Message Length: {}", config.chat.max_message_length);
            println!(
                "  Max Room Name Length: {}",
                config.chat.max_room_name_length
            );
            println!("  Max Users Per Room: {}", config.chat.max_users_per_room);
            println!("  Default Rooms:");
            for room in &config.chat.default_rooms {
                println!("    - {} ({})", room.name, room.id);
            }

            println!("\nLogging Configuration:");
            println!("  Level: {}", config.logging.level);
            println!("  Format: {}", config.logging.format);

            println!("\nAdmin Configuration:");
            println!("  Auto-create: {}", config.admin.auto_create);
            println!("  Admin Users:");
            for user in &config.admin.users {
                println!("    - {} ({:?})", user.username, user.permissions);
            }

            println!("\nRate Limiting:");
            println!("  Enabled: {}", config.rate_limit.enabled);
            if config.rate_limit.enabled {
                println!(
                    "  Messages/minute: {}",
                    config.rate_limit.messages_per_minute
                );
                println!("  Connections/IP: {}", config.rate_limit.connections_per_ip);
            }

            println!("\nSocket Address: {}", config.socket_addr());
            println!("Log Filter: {}", config.log_filter());
        }
        Err(e) => {
            eprintln!("✗ Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    }
}
