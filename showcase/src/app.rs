use std::io::{Stdout, stdout};
use std::sync::mpsc::Receiver;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::ExecutableCommand;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{
    prelude::*,
    widgets::{block::*, *},
};
use crate::click_api::ClickApi;

pub struct App {
    click_api: ClickApi,
    vm_packet_received_receiver: Receiver<()>,
    post_filtering_packet_received_receiver: Receiver<()>,

    terminal: Terminal<CrosstermBackend<Stdout>>,

    ui_state: UiState,
    stopped: bool,
}

const TICKS_PER_PACKET_HISTORY_MOVEMENT: u64 = 30;
const PACKET_HISTORY_SIZE: usize = 120;

struct UiState {
    ticks: u64,

    sent_packet_in_window: bool,
    sent_packets_count: u64,
    sent_packets: Vec<u64>,

    vm_received_packet_in_window: bool,
    vm_received_packets_count: u64,
    vm_received_packets: Vec<u64>,

    post_filtering_received_packet_in_window: bool,
    post_filtering_received_packets_count: u64,
    post_filtering_received_packets: Vec<u64>,

    selected_controls_button: Option<Button>,
    controls_state: ListState,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Button {
    AllowPackets,
    BlockPackets,
    SendPacket,
}

impl Button {
    fn all() -> [Button; 3] {
        [
            Button::AllowPackets,
            Button::BlockPackets,
            Button::SendPacket,
        ]
    }
}

struct ControlButton {
    button: Button,
    selected: bool,
}

impl<'a> Into<ListItem<'a>> for ControlButton {
    fn into(self) -> ListItem<'a> {
        let label = match self.button {
            Button::AllowPackets => "Allow all packets",
            Button::BlockPackets => "Block all packets",
            Button::SendPacket => "Send packet",
        };

        let text = if self.selected {
            Text::styled(format!("{label} (*)"), Style::default().add_modifier(Modifier::ITALIC))
        } else {
            Text::raw(label)
        };

        ListItem::new(text)
    }
}

impl App {
    pub fn new(
        click_api: ClickApi,
        vm_packet_received_receiver: Receiver<()>,
        post_filtering_packet_received_receiver: Receiver<()>,
    ) -> eyre::Result<Self> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.clear()?;

        Ok(Self {
            click_api,
            vm_packet_received_receiver,
            post_filtering_packet_received_receiver,
            terminal,
            ui_state: UiState {
                ticks: 0,
                vm_received_packet_in_window: false,
                vm_received_packets_count: 0,
                vm_received_packets: vec![0; PACKET_HISTORY_SIZE],
                post_filtering_received_packet_in_window: false,
                post_filtering_received_packets_count: 0,
                post_filtering_received_packets: vec![0; PACKET_HISTORY_SIZE],
                sent_packet_in_window: false,
                sent_packets_count: 0,
                sent_packets: vec![0; PACKET_HISTORY_SIZE],
                selected_controls_button: Some(Button::AllowPackets),
                controls_state: ListState::default().with_selected(Some(0)),
            },
            stopped: false,
        })
    }

    pub fn run(&mut self) -> eyre::Result<()> {
        loop {
            if self.stopped {
                break;
            }

            self.draw_ui()?;
            self.handle_events()?;
            self.update_state()?;

            self.ui_state.ticks += 1;
        }

        Ok(())
    }

    fn draw_ui(&mut self) -> eyre::Result<()> {
        self.terminal.draw(|frame| {
            // Create a layout with a horizontal split
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ]).split(frame.size());

            // Draw logs pane
            Self::draw_logs_pane(frame, layout[0]);

            // Draw right pane: controls (top) & stats (bottom)
            let right_pane_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length((Button::all().len() + 2 /* border */ + 2 /* padding */) as u16),
                    Constraint::Fill(1)
                ]).split(layout[1]);

            Self::draw_controls_pane(&mut self.ui_state, frame, right_pane_layout[0]);

            // Draw stats pane
            Self::draw_stats_pane(&mut self.ui_state, frame, right_pane_layout[1]);
        })?;

        Ok(())
    }

    fn draw_controls_pane(ui_state: &mut UiState, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Controls")
            .borders(Borders::ALL)
            .title_style(Style::default().fg(Color::Yellow))
            .border_style(Style::default().fg(Color::Yellow))
            .padding(Padding::uniform(1));

        let list = List::new(Button::all().map(|button| ControlButton {
            button,
            selected: ui_state.selected_controls_button == Some(button),
        }))
            .block(block)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::TopToBottom);

        frame.render_stateful_widget(
            list,
            area,
            &mut ui_state.controls_state,
        );
    }

    fn draw_stats_pane(ui_state: &mut UiState, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Stats")
            .borders(Borders::ALL)
            .title_style(Style::default().fg(Color::Yellow))
            .border_style(Style::default().fg(Color::Yellow))
            .padding(Padding::uniform(1));

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // sent packets
                Constraint::Length(1), // margin
                Constraint::Length(2), // received packets in VM
                Constraint::Length(1), // margin
                Constraint::Length(2), // received packets in server
            ]).split(block.inner(area));

        let sent_packets_block = Block::default()
            .title(format!("Sent packets ({})", ui_state.sent_packets_count))
            .borders(Borders::NONE)
            .title_style(Style::default().fg(Color::White).add_modifier(Modifier::ITALIC));

        let data = ui_state.sent_packets.iter().rev().copied().collect::<Vec<_>>();
        let sent_packets_sparkline = Sparkline::default()
            .block(sent_packets_block)
            .data(&data)
            .style(Style::default().fg(Color::Cyan).bg(Color::Gray));

        let vm_received_packets_block = Block::default()
            .title(format!("Received packets in VM ({})", ui_state.vm_received_packets_count))
            .borders(Borders::NONE)
            .title_style(Style::default().fg(Color::White).add_modifier(Modifier::ITALIC));

        let data = ui_state.vm_received_packets.iter().copied().rev().collect::<Vec<_>>();
        let vm_received_packets_sparkline = Sparkline::default()
            .block(vm_received_packets_block)
            .data(&data)
            .style(Style::default().fg(Color::Blue).bg(Color::Gray));

        let post_filtering_received_packets_block = Block::default()
            .title(format!("Non-dropped packets ({})", ui_state.post_filtering_received_packets_count))
            .borders(Borders::NONE)
            .title_style(Style::default().fg(Color::White).add_modifier(Modifier::ITALIC));

        let data = ui_state.post_filtering_received_packets.iter().copied().rev().collect::<Vec<_>>();
        let post_filtering_received_packets_sparkline = Sparkline::default()
            .block(post_filtering_received_packets_block)
            .data(&data)
            .style(Style::default().fg(Color::LightGreen).bg(Color::Gray));

        frame.render_widget(block, area);
        frame.render_widget(sent_packets_sparkline, layout[0]);
        frame.render_widget(vm_received_packets_sparkline, layout[2]);
        frame.render_widget(post_filtering_received_packets_sparkline, layout[4]);
    }

    fn draw_logs_pane(frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("VM Logs")
            .borders(Borders::ALL)
            .title_style(Style::default().fg(Color::Yellow))
            .border_style(Style::default().fg(Color::Yellow))
            .padding(Padding::uniform(1));

        let logger = tui_logger::TuiLoggerWidget::default()
            .block(block)
            .style(Style::default().fg(Color::White))
            .output_level(None)
            .output_timestamp(None)
            .output_target(false)
            .output_file(false)
            .output_line(false);

        frame.render_widget(logger, area);
    }

    fn handle_events(&mut self) -> eyre::Result<()> {
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('q') => {
                        self.stopped = true;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let i = match self.ui_state.controls_state.selected() {
                            Some(i) => {
                                if i >= Button::all().len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };

                        self.ui_state.controls_state.select(Some(i));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let i = match self.ui_state.controls_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    Button::all().len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };

                        self.ui_state.controls_state.select(Some(i));
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = self.ui_state.controls_state.selected() {
                            let button = Button::all()[selected];

                            match button {
                                Button::AllowPackets => {
                                    self.click_api.reconfigure(1, "pass")?;
                                    self.ui_state.selected_controls_button = Some(Button::AllowPackets);
                                }
                                Button::BlockPackets => {
                                    self.click_api.reconfigure(1, "drop")?;
                                    self.ui_state.selected_controls_button = Some(Button::BlockPackets);
                                }
                                Button::SendPacket if !self.ui_state.sent_packet_in_window => {
                                    self.click_api.send_data_packet()?;

                                    self.ui_state.sent_packets_count += 1;
                                    self.ui_state.sent_packets.push(1);
                                    self.ui_state.sent_packet_in_window = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn update_state(&mut self) -> eyre::Result<()> {
        // Update sent packets
        let packet_history_movement_tick = self.ui_state.ticks % TICKS_PER_PACKET_HISTORY_MOVEMENT == 0;
        if packet_history_movement_tick {
            if !self.ui_state.sent_packet_in_window {
                self.ui_state.sent_packets.push(0);
            }

            self.ui_state.sent_packet_in_window = false;

            if self.ui_state.sent_packets.len() > PACKET_HISTORY_SIZE {
                self.ui_state.sent_packets.remove(0);
            }
        }

        // Update received packets in VM
        self.vm_packet_received_receiver.try_iter().for_each(|_| {
            self.ui_state.vm_received_packets_count += 1;
            if !self.ui_state.vm_received_packet_in_window {
                self.ui_state.vm_received_packets.push(1);
                self.ui_state.vm_received_packet_in_window = true;
            }
        });

        if packet_history_movement_tick {
            if !self.ui_state.vm_received_packet_in_window {
                self.ui_state.vm_received_packets.push(0);
            }

            self.ui_state.vm_received_packet_in_window = false;

            if self.ui_state.vm_received_packets.len() > PACKET_HISTORY_SIZE {
                self.ui_state.vm_received_packets.remove(0);
            }
        }

        // Update received packets in server
        self.post_filtering_packet_received_receiver.try_iter().for_each(|_| {
            self.ui_state.post_filtering_received_packets_count += 1;
            if !self.ui_state.post_filtering_received_packet_in_window {
                self.ui_state.post_filtering_received_packets.push(1);
                self.ui_state.post_filtering_received_packet_in_window = true;
            }
        });

        if packet_history_movement_tick {
            if !self.ui_state.post_filtering_received_packet_in_window {
                self.ui_state.post_filtering_received_packets.push(0);
            }

            self.ui_state.post_filtering_received_packet_in_window = false;

            if self.ui_state.post_filtering_received_packets.len() > PACKET_HISTORY_SIZE {
                self.ui_state.post_filtering_received_packets.remove(0);
            }
        }

        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        stdout().execute(LeaveAlternateScreen).unwrap();
        disable_raw_mode().unwrap();
    }
}