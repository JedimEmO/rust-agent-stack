use crate::app::{AppScreen, AppState, AuthField};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

pub fn draw(frame: &mut Frame, app: &AppState) {
    match &app.screen {
        AppScreen::Login => draw_login_screen(frame, app),
        AppScreen::Register => draw_register_screen(frame, app),
        AppScreen::RoomList => draw_room_list_screen(frame, app),
        AppScreen::Chat { room_name, .. } => draw_chat_screen(frame, app, room_name),
    }

    // Draw error popup if there's an error
    if let Some(error) = &app.error_message {
        draw_error_popup(frame, error);
    }
}

fn draw_login_screen(frame: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(frame.area());

    let auth_block = Block::default()
        .title(" Login ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(auth_block.clone(), chunks[1]);
    
    let inner_area = auth_block.inner(chunks[1]);
    let auth_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // Username field
            Constraint::Length(1),  // Spacing
            Constraint::Length(2),  // Password field
            Constraint::Length(1),  // Spacing
            Constraint::Min(0),     // Instructions
        ])
        .split(inner_area);

    // Username field
    let username_style = if app.auth_field_focus == AuthField::Username {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let username_field = Paragraph::new(format!("Username: {}", app.auth_username_input))
        .style(username_style);
    frame.render_widget(username_field, auth_chunks[0]);
    
    // Add underline for username field
    let username_underline = Paragraph::new("─".repeat(auth_chunks[0].width as usize))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(username_underline, Rect {
        x: auth_chunks[0].x,
        y: auth_chunks[0].y + 1,
        width: auth_chunks[0].width,
        height: 1,
    });

    // Password field
    let password_style = if app.auth_field_focus == AuthField::Password {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let password_display = "*".repeat(app.auth_password_input.len());
    let password_field = Paragraph::new(format!("Password: {}", password_display))
        .style(password_style);
    frame.render_widget(password_field, auth_chunks[2]);
    
    // Add underline for password field
    let password_underline = Paragraph::new("─".repeat(auth_chunks[2].width as usize))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(password_underline, Rect {
        x: auth_chunks[2].x,
        y: auth_chunks[2].y + 1,
        width: auth_chunks[2].width,
        height: 1,
    });

    // Instructions
    let instructions = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" to switch fields | "),
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to login"),
        ]),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("Ctrl+R", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw(" to create a new account | "),
            Span::styled("Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" to quit"),
        ]),
    ])
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center);
    frame.render_widget(instructions, auth_chunks[4]);

    // Set cursor position
    match app.auth_field_focus {
        AuthField::Username => {
            frame.set_cursor_position((
                auth_chunks[0].x + 10 + app.auth_username_input.len() as u16,
                auth_chunks[0].y,
            ));
        }
        AuthField::Password => {
            frame.set_cursor_position((
                auth_chunks[2].x + 10 + app.auth_password_input.len() as u16,
                auth_chunks[2].y,
            ));
        }
    }
}

fn draw_register_screen(frame: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(frame.area());

    let auth_block = Block::default()
        .title(" Create New Account ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(Color::Green));

    frame.render_widget(auth_block.clone(), chunks[1]);
    
    let inner_area = auth_block.inner(chunks[1]);
    let auth_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // Username field
            Constraint::Length(1),  // Spacing
            Constraint::Length(2),  // Password field
            Constraint::Length(1),  // Spacing
            Constraint::Min(0),     // Instructions
        ])
        .split(inner_area);

    // Username field
    let username_style = if app.auth_field_focus == AuthField::Username {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let username_field = Paragraph::new(format!("Username: {}", app.auth_username_input))
        .style(username_style);
    frame.render_widget(username_field, auth_chunks[0]);
    
    // Add underline for username field
    let username_underline = Paragraph::new("─".repeat(auth_chunks[0].width as usize))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(username_underline, Rect {
        x: auth_chunks[0].x,
        y: auth_chunks[0].y + 1,
        width: auth_chunks[0].width,
        height: 1,
    });

    // Password field
    let password_style = if app.auth_field_focus == AuthField::Password {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    
    let password_display = "*".repeat(app.auth_password_input.len());
    let password_field = Paragraph::new(format!("Password: {}", password_display))
        .style(password_style);
    frame.render_widget(password_field, auth_chunks[2]);
    
    // Add underline for password field
    let password_underline = Paragraph::new("─".repeat(auth_chunks[2].width as usize))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(password_underline, Rect {
        x: auth_chunks[2].x,
        y: auth_chunks[2].y + 1,
        width: auth_chunks[2].width,
        height: 1,
    });

    // Instructions
    let instructions = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" to switch fields | "),
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to create account"),
        ]),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" to go back to login"),
        ]),
    ])
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center);
    frame.render_widget(instructions, auth_chunks[4]);

    // Set cursor position
    match app.auth_field_focus {
        AuthField::Username => {
            frame.set_cursor_position((
                auth_chunks[0].x + 10 + app.auth_username_input.len() as u16,
                auth_chunks[0].y,
            ));
        }
        AuthField::Password => {
            frame.set_cursor_position((
                auth_chunks[2].x + 10 + app.auth_password_input.len() as u16,
                auth_chunks[2].y,
            ));
        }
    }
}

fn draw_room_list_screen(frame: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(frame.area());

    // Header
    let header = Paragraph::new(format!(" Welcome, {}! ", app.username.as_ref().unwrap_or(&"User".to_string())))
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Thick));
    frame.render_widget(header, chunks[0]);

    // Room list
    let room_items: Vec<ListItem> = app
        .rooms
        .iter()
        .enumerate()
        .map(|(i, room)| {
            let content = format!("{:>2}. {} ({} users)", i + 1, room.room_name, room.user_count);
            ListItem::new(content)
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let rooms_list = List::new(room_items)
        .block(
            Block::default()
                .title(" Available Rooms ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(rooms_list, chunks[1]);

    // Instructions
    let instructions = Paragraph::new(vec![
        Line::from("Press 1-9 to join a room | Press R to refresh | Press Q to quit"),
    ])
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(instructions, chunks[2]);
}

fn draw_chat_screen(frame: &mut Frame, app: &AppState, room_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(5)])
        .split(frame.area());

    // Header
    let header = Paragraph::new(format!(" {} - {} ", room_name, app.username.as_ref().unwrap_or(&"User".to_string())))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Thick));
    frame.render_widget(header, chunks[0]);

    // Messages area
    let messages_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    
    let messages_area = messages_block.inner(chunks[1]);
    frame.render_widget(messages_block, chunks[1]);

    // Render messages
    let messages: Vec<Line> = app
        .messages
        .iter()
        .filter(|msg| {
            if let Some((room_id, _)) = &app.current_room {
                &msg.room_id == room_id
            } else {
                false
            }
        })
        .flat_map(|msg| {
            vec![
                Line::from(vec![
                    Span::styled(
                        format!("[{}] ", msg.timestamp.format("%H:%M:%S")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{}: ", msg.username),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&msg.text),
                ]),
            ]
        })
        .collect();

    let messages_widget = Paragraph::new(messages)
        .wrap(Wrap { trim: true })
        .scroll((
            app.messages.len().saturating_sub(messages_area.height as usize) as u16,
            0,
        ));
    frame.render_widget(messages_widget, messages_area);

    // Input area with help text
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(1)])
        .split(chunks[2]);
    
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(" Type your message ");
    
    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .block(input_block);
    frame.render_widget(input, input_chunks[0]);

    // Help text
    let help_text = Paragraph::new("Press Esc to leave room | /quit to exit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help_text, input_chunks[1]);

    // Show cursor
    frame.set_cursor_position((input_chunks[0].x + 1 + app.input_buffer.len() as u16, input_chunks[0].y + 1));
}

fn draw_error_popup(frame: &mut Frame, error: &str) {
    let area = centered_rect(60, 20, frame.area());
    
    let popup_block = Block::default()
        .title(" Error ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().fg(Color::Red));

    let error_text = Paragraph::new(error)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White))
        .block(popup_block);

    frame.render_widget(Clear, area);
    frame.render_widget(error_text, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}