use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::audio::diagnost::indicator::VolumeIndicator;
use crate::audio::{LatencySnapshot, LatencyStats};
use crate::core::session_event::SessionEvent;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};
use tokio::sync::mpsc;

const SIGNAL_FLOOR_DBFS: f32 = -60.0;
const SIGNAL_ATTACK: Duration = Duration::from_millis(40);
const SIGNAL_RELEASE: Duration = Duration::from_millis(650);
const SIGNAL_KIND_WIDTH: u16 = 4;

#[derive(Debug, Clone, Copy)]
struct SignalLevel {
    indicator: VolumeIndicator,
    display_peak_dbfs: f32,
    display_rms_dbfs: f32,
    smoothed_at: Instant,
}

impl SignalLevel {
    fn update(&mut self, indicator: VolumeIndicator) {
        self.indicator = indicator;
    }

    fn smooth(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.smoothed_at);

        if elapsed.is_zero() {
            return;
        }

        self.display_peak_dbfs = smooth_dbfs(
            self.display_peak_dbfs,
            amplitude_to_dbfs(self.indicator.peak),
            elapsed,
        );
        self.display_rms_dbfs = smooth_dbfs(self.display_rms_dbfs, self.indicator.dbfs, elapsed);

        self.smoothed_at = now;
    }

    fn ratio(&self) -> f64 {
        dbfs_to_ratio(self.display_peak_dbfs)
    }

    fn label(&self) -> String {
        if self.display_peak_dbfs <= SIGNAL_FLOOR_DBFS || !self.display_rms_dbfs.is_finite() {
            "0 dB".to_string()
        } else {
            format!("{:.0} dB", self.display_rms_dbfs)
        }
    }
}

impl Default for SignalLevel {
    fn default() -> Self {
        Self {
            indicator: VolumeIndicator {
                rms: 0.0,
                peak: 0.0,
                dbfs: f32::NEG_INFINITY,
            },
            display_peak_dbfs: SIGNAL_FLOOR_DBFS,
            display_rms_dbfs: SIGNAL_FLOOR_DBFS,
            smoothed_at: Instant::now(),
        }
    }
}

pub struct ConsoleApp {
    direct_rx: mpsc::UnboundedReceiver<SessionEvent>,
    back_rx: mpsc::UnboundedReceiver<SessionEvent>,

    direct_input_indicator_rx: mpsc::Receiver<VolumeIndicator>,
    direct_output_indicator_rx: mpsc::Receiver<VolumeIndicator>,
    back_input_indicator_rx: mpsc::Receiver<VolumeIndicator>,
    back_output_indicator_rx: mpsc::Receiver<VolumeIndicator>,

    direct_stats: Arc<LatencyStats>,
    back_stats: Arc<LatencyStats>,

    direct_text: String,
    back_text: String,

    direct_input_text: String,
    back_input_text: String,

    direct_latency: LatencySnapshot,
    back_latency: LatencySnapshot,

    direct_input_signal: SignalLevel,
    direct_output_signal: SignalLevel,
    back_input_signal: SignalLevel,
    back_output_signal: SignalLevel,

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
        direct_input_indicator_rx: mpsc::Receiver<VolumeIndicator>,
        direct_output_indicator_rx: mpsc::Receiver<VolumeIndicator>,
        back_input_indicator_rx: mpsc::Receiver<VolumeIndicator>,
        back_output_indicator_rx: mpsc::Receiver<VolumeIndicator>,
    ) -> Self {
        Self {
            direct_rx,
            back_rx,
            direct_input_indicator_rx,
            direct_output_indicator_rx,
            back_input_indicator_rx,
            back_output_indicator_rx,
            direct_stats,
            back_stats,
            direct_text: String::new(),
            back_text: String::new(),
            direct_input_text: String::new(),
            back_input_text: String::new(),
            direct_latency: LatencySnapshot::default(),
            back_latency: LatencySnapshot::default(),
            direct_input_signal: SignalLevel::default(),
            direct_output_signal: SignalLevel::default(),
            back_input_signal: SignalLevel::default(),
            back_output_signal: SignalLevel::default(),
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
            self.read_indicators();
            self.smooth_indicators();

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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
            SessionEvent::InputText(text) => self.direct_input_text.push_str(&text),
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
            SessionEvent::InputText(text) => self.back_input_text.push_str(&text),
            _ => {}
        }
    }

    fn read_indicators(&mut self) {
        while let Ok(indicator) = self.direct_input_indicator_rx.try_recv() {
            self.direct_input_signal.update(indicator);
        }
        while let Ok(indicator) = self.direct_output_indicator_rx.try_recv() {
            self.direct_output_signal.update(indicator);
        }
        while let Ok(indicator) = self.back_input_indicator_rx.try_recv() {
            self.back_input_signal.update(indicator);
        }
        while let Ok(indicator) = self.back_output_indicator_rx.try_recv() {
            self.back_output_signal.update(indicator);
        }
    }

    fn smooth_indicators(&mut self) {
        let now = Instant::now();

        self.direct_input_signal.smooth(now);
        self.direct_output_signal.smooth(now);
        self.back_input_signal.smooth(now);
        self.back_output_signal.smooth(now);
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Min(3),
                Constraint::Length(4),
            ])
            .split(f.area());

        let input_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);

        let direct_input_block = Block::default().borders(Borders::ALL).title(" RU Input ");
        let direct_input_para = Paragraph::new(self.direct_input_text.as_str())
            .block(direct_input_block)
            .wrap(Wrap { trim: false })
            .scroll((
                self.direct_input_text
                    .lines()
                    .count()
                    .saturating_sub(input_chunks[0].height as usize - 2) as u16,
                0,
            ));
        f.render_widget(direct_input_para, input_chunks[0]);

        let back_input_block = Block::default().borders(Borders::ALL).title(" EN Input ");
        let back_input_para = Paragraph::new(self.back_input_text.as_str())
            .block(back_input_block)
            .wrap(Wrap { trim: false })
            .scroll((
                self.back_input_text
                    .lines()
                    .count()
                    .saturating_sub(input_chunks[1].height as usize - 2) as u16,
                0,
            ));
        f.render_widget(back_input_para, input_chunks[1]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let direct_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(3)])
            .split(main_chunks[0]);

        let back_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(3)])
            .split(main_chunks[1]);

        self.render_signal_pair(
            f,
            direct_chunks[0],
            " RU -> EN Signal ",
            &self.direct_input_signal,
            &self.direct_output_signal,
        );

        self.render_signal_pair(
            f,
            back_chunks[0],
            " EN -> RU Signal ",
            &self.back_input_signal,
            &self.back_output_signal,
        );

        let direct_block = Block::default().borders(Borders::ALL).title(" RU -> EN ");
        let direct_para = Paragraph::new(self.direct_text.as_str())
            .block(direct_block)
            .wrap(Wrap { trim: false })
            .scroll((
                self.direct_text
                    .lines()
                    .count()
                    .saturating_sub(direct_chunks[1].height as usize - 2) as u16,
                0,
            ));
        f.render_widget(direct_para, direct_chunks[1]);

        let back_block = Block::default().borders(Borders::ALL).title(" EN -> RU ");
        let back_para = Paragraph::new(self.back_text.as_str())
            .block(back_block)
            .wrap(Wrap { trim: false })
            .scroll((
                self.back_text
                    .lines()
                    .count()
                    .saturating_sub(back_chunks[1].height as usize - 2) as u16,
                0,
            ));
        f.render_widget(back_para, back_chunks[1]);

        let status_text = format!(
            "Direct (RU-EN): {} | In: {:.1}ms | Out: {:.1}ms | Lost: (I:{}, N:{}, O:{})\n\
             Back   (EN-RU): {} | In: {:.1}ms | Out: {:.1}ms | Lost: (I:{}, N:{}, O:{})\n\
             Press 'q' or 'Esc' to exit",
            self.direct_status,
            self.direct_latency.input_total.last_ms,
            self.direct_latency.output_total.last_ms,
            self.direct_latency.dropped_input,
            self.direct_latency.dropped_network,
            self.direct_latency.dropped_output,
            self.back_status,
            self.back_latency.input_total.last_ms,
            self.back_latency.output_total.last_ms,
            self.back_latency.dropped_input,
            self.back_latency.dropped_network,
            self.back_latency.dropped_output,
        );
        let status_block = Block::default()
            .borders(Borders::ALL)
            .title(" Status & Latency ");
        let status_para = Paragraph::new(status_text).block(status_block);
        f.render_widget(status_para, chunks[2]);
    }

    fn render_signal_pair(
        &self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        title: &str,
        input: &SignalLevel,
        output: &SignalLevel,
    ) {
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);

        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        self.render_signal_gauge(f, rows[0], "IN ", input);
        self.render_signal_gauge(f, rows[1], "OUT", output);
    }

    fn render_signal_gauge(
        &self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        name: &str,
        signal: &SignalLevel,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(SIGNAL_KIND_WIDTH), Constraint::Min(1)])
            .split(area);
        let kind = Paragraph::new(name).style(Style::default().fg(Color::Gray));
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(signal_color(signal.ratio())))
            .ratio(signal.ratio())
            .label(signal.label());

        f.render_widget(kind, chunks[0]);
        f.render_widget(gauge, chunks[1]);
    }
}

fn amplitude_to_dbfs(amplitude: f32) -> f32 {
    if amplitude > 0.0 {
        20.0 * amplitude.log10()
    } else {
        f32::NEG_INFINITY
    }
}

fn smooth_dbfs(current: f32, target: f32, elapsed: Duration) -> f32 {
    let target = finite_or_floor(target);
    let current = finite_or_floor(current);
    let time_constant = if target > current {
        SIGNAL_ATTACK
    } else {
        SIGNAL_RELEASE
    };
    let alpha = 1.0 - (-elapsed.as_secs_f32() / time_constant.as_secs_f32()).exp();

    current + (target - current) * alpha
}

fn finite_or_floor(dbfs: f32) -> f32 {
    if dbfs.is_finite() {
        dbfs.max(SIGNAL_FLOOR_DBFS)
    } else {
        SIGNAL_FLOOR_DBFS
    }
}

fn dbfs_to_ratio(dbfs: f32) -> f64 {
    if !dbfs.is_finite() || dbfs <= SIGNAL_FLOOR_DBFS {
        return 0.0;
    }

    ((dbfs - SIGNAL_FLOOR_DBFS) / -SIGNAL_FLOOR_DBFS) as f64
}

fn signal_color(ratio: f64) -> Color {
    if ratio >= 0.82 {
        Color::Red
    } else if ratio >= 0.55 {
        Color::Yellow
    } else {
        Color::Green
    }
}
