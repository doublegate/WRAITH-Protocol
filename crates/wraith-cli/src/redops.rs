use clap::Subcommand;
use anyhow::Result;
use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, List, ListItem},
};
use std::io::stdout;

pub mod proto {
    tonic::include_proto!("wraith.redops");
}
use proto::operator_service_client::OperatorServiceClient;
use proto::ListAttackChainsRequest;

#[derive(Subcommand)]
pub enum RedOpsCommands {
    /// Launch the RedOps TUI Console
    Console {
        /// Team Server URL
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },
}

pub async fn run(command: RedOpsCommands) -> Result<()> {
    match command {
        RedOpsCommands::Console { server } => {
            run_tui(&server).await?;
        }
    }
    Ok(())
}

async fn run_tui(server: &str) -> Result<()> {
    // Setup terminal
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Connect to server (simplified)
    let mut client = OperatorServiceClient::connect(server.to_string()).await?;

    // Fetch chains
    let response = client.list_attack_chains(tonic::Request::new(ListAttackChainsRequest {
        page_size: 100,
        page_token: "".to_string(),
    })).await?;
    let chains = response.into_inner().chains;

    let mut should_quit = false;
    let mut selected_chain_idx = 0;
    let mut view_mode = 0; // 0: Tree, 1: Flowchart, 2: Mermaid

    while !should_quit {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(f.size());

            // Sidebar: Chain List
            let items: Vec<ListItem> = chains.iter().map(|c| ListItem::new(c.name.clone())).collect();
            let list = List::new(items)
                .block(Block::default().title("Attack Chains").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Red))
                .highlight_symbol("> ");
            
            // We need state for list selection, simplified here by just rendering
            // In a real app we'd use ListState.
            f.render_widget(list, chunks[0]);

            // Main View: Visualization
            let title = match view_mode {
                0 => "View: Tree",
                1 => "View: ASCII Flowchart",
                2 => "View: Mermaid",
                _ => "View",
            };

            let content = if chains.is_empty() {
                "No chains found".to_string()
            } else {
                let chain = &chains[selected_chain_idx];
                match view_mode {
                    0 => render_tree(chain),
                    1 => render_flowchart(chain),
                    2 => render_mermaid(chain),
                    _ => "".to_string(),
                }
            };

            let p = Paragraph::new(content)
                .block(Block::default().title(title).borders(Borders::ALL));
            f.render_widget(p, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => should_quit = true,
                        KeyCode::Tab => view_mode = (view_mode + 1) % 3,
                        KeyCode::Down => {
                            if !chains.is_empty() {
                                selected_chain_idx = (selected_chain_idx + 1) % chains.len();
                            }
                        }
                        KeyCode::Up => {
                            if !chains.is_empty() {
                                selected_chain_idx = (selected_chain_idx + chains.len() - 1) % chains.len();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn render_tree(chain: &proto::AttackChain) -> String {
    let mut out = String::new();
    out.push_str(&format!("📦 {}\n", chain.name));
    for step in &chain.steps {
        out.push_str(&format!(" ┣━━ 📝 Step {}: {} ({})\n", step.step_order, step.technique_id, step.command_type));
        out.push_str(&format!(" ┃    └─ {}\n", step.description));
    }
    out
}

fn render_flowchart(chain: &proto::AttackChain) -> String {
    let mut out = String::new();
    out.push_str("START\n  │\n  ▼\n");
    for step in &chain.steps {
        out.push_str(&format!("┌──────────────────────┐\n"));
        out.push_str(&format!("│ {:<20} │\n", step.technique_id));
        out.push_str(&format!("└──────────────────────┘\n"));
        out.push_str("  │\n  ▼\n");
    }
    out.push_str("END\n");
    out
}

fn render_mermaid(chain: &proto::AttackChain) -> String {
    let mut out = String::new();
    out.push_str("graph TD\n");
    out.push_str("    Start((Start)) --> Step1\n");
    for (i, step) in chain.steps.iter().enumerate() {
        let current = format!("Step{}", i + 1);
        let next = if i + 1 < chain.steps.len() { format!("Step{}", i + 2) } else { "End".to_string() };
        out.push_str(&format!("    {}[{}] --> {}\n", current, step.technique_id, next));
    }
    out
}
