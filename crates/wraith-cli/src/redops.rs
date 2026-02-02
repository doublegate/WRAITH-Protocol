use anyhow::Result;
use clap::Subcommand;
use crossterm::{
    ExecutableCommand,
    event::{self, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io::stdout;

pub mod proto {
    tonic::include_proto!("wraith.redops");
}
use proto::ListAttackChainsRequest;
use proto::operator_service_client::OperatorServiceClient;

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
    let response = client
        .list_attack_chains(tonic::Request::new(ListAttackChainsRequest {
            page_size: 100,
            page_token: "".to_string(),
        }))
        .await?;
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
            let items: Vec<ListItem> = chains
                .iter()
                .map(|c| ListItem::new(c.name.clone()))
                .collect();
            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Attack Chains")
                        .borders(Borders::ALL),
                )
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

            let p =
                Paragraph::new(content).block(Block::default().title(title).borders(Borders::ALL));
            f.render_widget(p, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))?
            && let event::Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
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

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

pub(crate) fn render_tree(chain: &proto::AttackChain) -> String {
    let mut out = String::new();
    out.push_str(&format!("📦 {}\n", chain.name));
    for step in &chain.steps {
        out.push_str(&format!(
            " ┣━━ 📝 Step {}: {} ({})\n",
            step.step_order, step.technique_id, step.command_type
        ));
        out.push_str(&format!(" ┃    └─ {}\n", step.description));
    }
    out
}

pub(crate) fn render_flowchart(chain: &proto::AttackChain) -> String {
    let mut out = String::new();
    out.push_str("START\n  │\n  ▼\n");
    for step in &chain.steps {
        out.push_str("┌──────────────────────┐\n");
        out.push_str(&format!("│ {:<20} │\n", step.technique_id));
        out.push_str("└──────────────────────┘\n");
        out.push_str("  │\n  ▼\n");
    }
    out.push_str("END\n");
    out
}

pub(crate) fn render_mermaid(chain: &proto::AttackChain) -> String {
    let mut out = String::new();
    out.push_str("graph TD\n");
    out.push_str("    Start((Start)) --> Step1\n");
    for (i, step) in chain.steps.iter().enumerate() {
        let current = format!("Step{}", i + 1);
        let next = if i + 1 < chain.steps.len() {
            format!("Step{}", i + 2)
        } else {
            "End".to_string()
        };
        out.push_str(&format!(
            "    {}[{}] --> {}\n",
            current, step.technique_id, next
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::{AttackChain, ChainStep};

    fn make_step(order: i32, technique: &str, cmd_type: &str, desc: &str) -> ChainStep {
        ChainStep {
            id: format!("step-{order}"),
            chain_id: "chain-1".to_string(),
            step_order: order,
            technique_id: technique.to_string(),
            command_type: cmd_type.to_string(),
            payload: String::new(),
            description: desc.to_string(),
        }
    }

    fn make_chain(name: &str, steps: Vec<ChainStep>) -> AttackChain {
        AttackChain {
            id: "chain-1".to_string(),
            name: name.to_string(),
            description: "Test chain".to_string(),
            steps,
            created_at: None,
            updated_at: None,
        }
    }

    // ── RedOpsCommands parsing ──────────────────────────────────────────

    #[test]
    fn test_redops_console_command_default_server() {
        use clap::Parser;

        // RedOpsCommands is a subcommand; we test via the parent CLI
        let cli = crate::Cli::parse_from(["wraith", "red-ops", "console"]);
        match cli.command {
            crate::Commands::RedOps(RedOpsCommands::Console { server }) => {
                assert_eq!(server, "http://127.0.0.1:50051");
            }
            _ => panic!("Expected RedOps Console command"),
        }
    }

    #[test]
    fn test_redops_console_command_custom_server() {
        use clap::Parser;

        let cli = crate::Cli::parse_from([
            "wraith",
            "red-ops",
            "console",
            "--server",
            "http://10.0.0.1:9090",
        ]);
        match cli.command {
            crate::Commands::RedOps(RedOpsCommands::Console { server }) => {
                assert_eq!(server, "http://10.0.0.1:9090");
            }
            _ => panic!("Expected RedOps Console command"),
        }
    }

    // ── render_tree ─────────────────────────────────────────────────────

    #[test]
    fn test_render_tree_empty_chain() {
        let chain = make_chain("EmptyChain", vec![]);
        let output = render_tree(&chain);
        assert!(output.contains("EmptyChain"));
        assert!(!output.contains("Step"));
    }

    #[test]
    fn test_render_tree_single_step() {
        let chain = make_chain(
            "SingleStep",
            vec![make_step(1, "T1059.001", "powershell", "Run PS")],
        );
        let output = render_tree(&chain);
        assert!(output.contains("SingleStep"));
        assert!(output.contains("T1059.001"));
        assert!(output.contains("powershell"));
        assert!(output.contains("Run PS"));
    }

    #[test]
    fn test_render_tree_multiple_steps() {
        let chain = make_chain(
            "MultiStep",
            vec![
                make_step(1, "T1059.001", "powershell", "Initial access"),
                make_step(2, "T1055.012", "inject", "Process hollowing"),
                make_step(3, "T1021.002", "smb", "Lateral movement"),
            ],
        );
        let output = render_tree(&chain);
        assert!(output.contains("T1059.001"));
        assert!(output.contains("T1055.012"));
        assert!(output.contains("T1021.002"));
        // Check ordering preserved
        let idx1 = output.find("T1059.001").unwrap();
        let idx2 = output.find("T1055.012").unwrap();
        let idx3 = output.find("T1021.002").unwrap();
        assert!(idx1 < idx2);
        assert!(idx2 < idx3);
    }

    // ── render_flowchart ────────────────────────────────────────────────

    #[test]
    fn test_render_flowchart_empty_chain() {
        let chain = make_chain("Empty", vec![]);
        let output = render_flowchart(&chain);
        assert!(output.contains("START"));
        assert!(output.contains("END"));
    }

    #[test]
    fn test_render_flowchart_single_step() {
        let chain = make_chain("One", vec![make_step(1, "T1059", "shell", "Execute")]);
        let output = render_flowchart(&chain);
        assert!(output.contains("START"));
        assert!(output.contains("T1059"));
        assert!(output.contains("END"));
    }

    #[test]
    fn test_render_flowchart_multiple_steps() {
        let chain = make_chain(
            "Multi",
            vec![
                make_step(1, "T1059", "shell", "Step 1"),
                make_step(2, "T1055", "inject", "Step 2"),
            ],
        );
        let output = render_flowchart(&chain);
        assert!(output.contains("T1059"));
        assert!(output.contains("T1055"));
        let idx1 = output.find("T1059").unwrap();
        let idx2 = output.find("T1055").unwrap();
        assert!(idx1 < idx2);
    }

    #[test]
    fn test_render_flowchart_has_box_borders() {
        let chain = make_chain("Boxes", vec![make_step(1, "T1059", "shell", "Exec")]);
        let output = render_flowchart(&chain);
        assert!(output.contains("┌──────────────────────┐"));
        assert!(output.contains("└──────────────────────┘"));
    }

    // ── render_mermaid ──────────────────────────────────────────────────

    #[test]
    fn test_render_mermaid_empty_chain() {
        let chain = make_chain("Empty", vec![]);
        let output = render_mermaid(&chain);
        assert!(output.contains("graph TD"));
        assert!(output.contains("Start((Start)) --> Step1"));
    }

    #[test]
    fn test_render_mermaid_single_step() {
        let chain = make_chain("One", vec![make_step(1, "T1059", "shell", "Exec")]);
        let output = render_mermaid(&chain);
        assert!(output.contains("graph TD"));
        assert!(output.contains("Step1[T1059] --> End"));
    }

    #[test]
    fn test_render_mermaid_multiple_steps() {
        let chain = make_chain(
            "Multi",
            vec![
                make_step(1, "T1059", "shell", "Step 1"),
                make_step(2, "T1055", "inject", "Step 2"),
                make_step(3, "T1021", "smb", "Step 3"),
            ],
        );
        let output = render_mermaid(&chain);
        assert!(output.contains("Step1[T1059] --> Step2"));
        assert!(output.contains("Step2[T1055] --> Step3"));
        assert!(output.contains("Step3[T1021] --> End"));
    }

    #[test]
    fn test_render_mermaid_two_steps() {
        let chain = make_chain(
            "Two",
            vec![
                make_step(1, "T1059", "shell", "A"),
                make_step(2, "T1055", "inject", "B"),
            ],
        );
        let output = render_mermaid(&chain);
        assert!(output.contains("Step1[T1059] --> Step2"));
        assert!(output.contains("Step2[T1055] --> End"));
    }
}
