use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tokio::sync::mpsc;
use crate::core::session_event::SessionEvent;
use crate::audio::{LatencySnapshot, LatencyStats};

pub struct ConsoleApp {
    direct_rx: mpsc::UnboundedReceiver<SessionEvent>,
    back_rx: mpsc::UnboundedReceiver<SessionEvent>,
    
    direct_stats: Arc<LatencyStats>,
    back_stats: Arc<LatencyStats>,
    
    direct_text: String,
    back_text: String,
    
    direct_latency: LatencySnapshot,
    back_latency: LatencySnapshot,
    
    direct_status: String,
    back_status: String,
    
    should_quit: bool,
}

impl ConsoleApp {
    pub fn new(
        direct_rx: mpsc::UnboundedReceiver<SessionEvent>,
        back_rx: mpsc::UnboundedReceiver<SessionEvent>,
        direct_stats: Arc<LatencyStats>,
        back_stats: Arc<LatencyStats>,
    ) -> Self {
        Self {
            direct_rx,
            back_rx,
            direct_stats,
            back_stats,
            direct_text: String::new(),
            back_text: String::new(),
            direct_latency: LatencySnapshot::default(),
            back_latency: LatencySnapshot::default(),
            direct_status: "IDLE".to_string(),
            back_status: "IDLE".to_string(),
            should_quit: false,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(50);

        while !self.should_quit {
            self.direct_latency = self.direct_stats.snapshot();
            self.back_latency = self.back_stats.snapshot();

            terminal.draw(|f| self.ui(f))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if let KeyCode::Char('q') | KeyCode::Esc = key.code {
                        self.should_quit = true;
                    }
                }
            }

            // Process events from lines
            while let Ok(event) = self.direct_rx.try_recv() {
                self.handle_direct_event(event);
            }
            while let Ok(event) = self.back_rx.try_recv() {
                self.handle_back_event(event);
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn handle_direct_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Text(delta) => self.direct_text.push_str(&delta),
            SessionEvent::SessionStarted(_) => self.direct_status = "STARTED".to_string(),
            SessionEvent::SessionConfigured(_) => self.direct_status = "READY".to_string(),
            SessionEvent::RequestStarted => self.direct_status = "LISTENING".to_string(),
            SessionEvent::ResponseStarted => self.direct_status = "THINKING".to_string(),
            SessionEvent::ResponseFinished => self.direct_status = "READY".to_string(),
            _ => {}
        }
    }

    fn handle_back_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Text(delta) => self.back_text.push_str(&delta),
            SessionEvent::SessionStarted(_) => self.back_status = "STARTED".to_string(),
            SessionEvent::SessionConfigured(_) => self.back_status = "READY".to_string(),
            SessionEvent::RequestStarted => self.back_status = "LISTENING".to_string(),
            SessionEvent::ResponseStarted => self.back_status = "THINKING".to_string(),
            SessionEvent::ResponseFinished => self.back_status = "READY".to_string(),
            _ => {}
        }
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(4),
            ])
            .split(f.area());

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(chunks[0]);

        let direct_block = Block::default()
            .borders(Borders::ALL)
            .title(" RU -> EN ");
        let direct_para = Paragraph::new(self.direct_text.as_str())
            .block(direct_block)
            .wrap(Wrap { trim: false })
            .scroll((self.direct_text.lines().count().saturating_sub(main_chunks[0].height as usize - 2) as u16, 0));
        f.render_widget(direct_para, main_chunks[0]);

        let back_block = Block::default()
            .borders(Borders::ALL)
            .title(" EN -> RU ");
        let back_para = Paragraph::new(self.back_text.as_str())
            .block(back_block)
            .wrap(Wrap { trim: false })
            .scroll((self.back_text.lines().count().saturating_sub(main_chunks[1].height as usize - 2) as u16, 0));
        f.render_widget(back_para, main_chunks[1]);

        let status_text = format!(
            "Direct (RU-EN): {} | In: {:.1}ms | Out: {:.1}ms\n\
             Back   (EN-RU): {} | In: {:.1}ms | Out: {:.1}ms\n\
             Press 'q' or 'Esc' to exit",
            self.direct_status, self.direct_latency.input_total.last_ms, self.direct_latency.output_total.last_ms,
            self.back_status, self.back_latency.input_total.last_ms, self.back_latency.output_total.last_ms,
        );
        let status_block = Block::default()
            .borders(Borders::ALL)
            .title(" Status & Latency ");
        let status_para = Paragraph::new(status_text)
            .block(status_block);
        f.render_widget(status_para, chunks[1]);
    }
}
