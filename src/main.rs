use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

const PADDLE_HEIGHT: i16 = 4;
const FPS: u64 = 60;
const FRAME_TIME: Duration = Duration::from_millis(1000 / FPS);
const WIN_SCORE: u32 = 7;
const STARTUP_DURATION: Duration = Duration::from_secs(2);

struct Ball {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}

struct Game {
    width: u16,
    height: u16,
    ball: Ball,
    paddle_a: f32, // Left paddle Y center (now f32 for smooth AI)
    paddle_b: f32, // Right paddle Y center
    score_a: u32,
    score_b: u32,
    fps: u32,                     // Current frames per second
    game_over: bool,
    winner: Option<String>,
    // AI state
    cpu_target_y: f32,
    cpu_last_update: Instant,
    rng: u64,                     // Simple PRNG state
}

impl Game {
    fn new(width: u16, height: u16) -> Self {
        // Seed RNG with system time for variety
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self {
            width,
            height,
            ball: Ball {
                x: (width / 2) as f32,
                y: (height / 2) as f32,
                dx: 0.5,
                dy: 0.2,
            },
            paddle_a: (height / 2) as f32,
            paddle_b: (height / 2) as f32,
            score_a: 0,
            score_b: 0,
            fps: 0,
            game_over: false,
            winner: None,
            cpu_target_y: (height / 2) as f32,
            cpu_last_update: Instant::now(),
            rng: seed,
        }
    }

    fn random_f(&mut self) -> f32 {
        // Fast PCG-like generator
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.rng as f64 / u64::MAX as f64) as f32
    }

    fn reset_ball(&mut self, direction: f32) {
        self.ball.x = (self.width / 2) as f32;
        self.ball.y = (self.height / 2) as f32;
        self.ball.dx = direction * 0.5;
        self.ball.dy = 0.2;
    }

    fn update(&mut self) {
        // Move ball
        self.ball.x += self.ball.dx;
        self.ball.y += self.ball.dy;

        // Ceiling and floor
        if self.ball.y <= 1.0 || self.ball.y >= (self.height - 1) as f32 {
            self.ball.dy = -self.ball.dy;
        }

        // Left paddle collision
        if self.ball.x <= 3.0 && self.ball.x >= 2.0 {
            let paddle_top = self.paddle_a as i16 - PADDLE_HEIGHT / 2;
            let paddle_bottom = self.paddle_a as i16 + PADDLE_HEIGHT / 2;
            if self.ball.y >= paddle_top as f32 && self.ball.y <= paddle_bottom as f32 {
                self.ball.dx = -self.ball.dx * 1.05;
                self.ball.dy += (self.ball.y - self.paddle_a) * 0.1;
            }
        }

        // Right paddle collision
        if self.ball.x >= (self.width - 3) as f32 && self.ball.x <= (self.width - 2) as f32 {
            let paddle_top = self.paddle_b as i16 - PADDLE_HEIGHT / 2;
            let paddle_bottom = self.paddle_b as i16 + PADDLE_HEIGHT / 2;
            if self.ball.y >= paddle_top as f32 && self.ball.y <= paddle_bottom as f32 {
                self.ball.dx = -self.ball.dx * 1.05;
                self.ball.dy += (self.ball.y - self.paddle_b) * 0.1;
            }
        }

        // Scoring & win condition
        if self.ball.x <= 0.0 {
            self.score_b += 1;
            if self.score_b >= WIN_SCORE {
                self.game_over = true;
                self.winner = Some("CPU".to_string());
            }
            self.reset_ball(1.0);
        } else if self.ball.x >= self.width as f32 {
            self.score_a += 1;
            if self.score_a >= WIN_SCORE {
                self.game_over = true;
                self.winner = Some("Player".to_string());
            }
            self.reset_ball(-1.0);
        }

        // --- CPU opponent (right paddle) ---
        // Update target every ~0.4s with a random offset to feel human
        if self.cpu_last_update.elapsed() > Duration::from_millis(400) {
            let offset = (self.random_f() * 6.0) - 3.0; // -3..3
            self.cpu_target_y = (self.ball.y + offset)
                .clamp(PADDLE_HEIGHT as f32 / 2.0, self.height as f32 - PADDLE_HEIGHT as f32 / 2.0);
            self.cpu_last_update = Instant::now();
        }
        // Move paddle towards target at a fixed speed (smooth, human-like)
        let diff = self.cpu_target_y - self.paddle_b;
        if diff.abs() < 0.5 {
            self.paddle_b = self.cpu_target_y;
        } else {
            self.paddle_b += diff.signum() * 1.0; // move 1 unit per frame
        }
        self.paddle_b = self.paddle_b.clamp(
            PADDLE_HEIGHT as f32 / 2.0,
            self.height as f32 - PADDLE_HEIGHT as f32 / 2.0,
        );
    }

    fn draw<W: Write>(&self, stdout: &mut W) -> std::io::Result<()> {
        // Clear screen (high-performance full clear)
        stdout.queue(crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;

        // 1. FPS box (top-left corner, fixed size)
        let fps_text = format!("FPS: {}", self.fps);
        stdout.queue(crossterm::cursor::MoveTo(0, 0))?;
        write!(stdout, "┌──────────┐")?;
        stdout.queue(crossterm::cursor::MoveTo(0, 1))?;
        write!(stdout, "│ {: <8} │", fps_text)?;
        stdout.queue(crossterm::cursor::MoveTo(0, 2))?;
        write!(stdout, "└──────────┘")?;

        // 2. Scores
        stdout.queue(crossterm::cursor::MoveTo(self.width / 4, 1))?;
        write!(stdout, "{}", self.score_a)?;
        stdout.queue(crossterm::cursor::MoveTo((self.width * 3) / 4, 1))?;
        write!(stdout, "{}", self.score_b)?;

        // 3. Center line
        for y in 0..self.height {
            if y % 2 == 0 {
                stdout.queue(crossterm::cursor::MoveTo(self.width / 2, y))?;
                write!(stdout, "|")?;
            }
        }

        // 4. Left paddle
        let a_start = (self.paddle_a as i16 - PADDLE_HEIGHT / 2).max(0);
        for i in 0..PADDLE_HEIGHT {
            stdout.queue(crossterm::cursor::MoveTo(2, (a_start + i) as u16))?;
            write!(stdout, "█")?;
        }

        // 5. Right paddle
        let b_start = (self.paddle_b as i16 - PADDLE_HEIGHT / 2).max(0);
        for i in 0..PADDLE_HEIGHT {
            stdout.queue(crossterm::cursor::MoveTo(self.width - 3, (b_start + i) as u16))?;
            write!(stdout, "█")?;
        }

        // 6. Ball
        stdout.queue(crossterm::cursor::MoveTo(
            self.ball.x as u16,
            self.ball.y as u16,
        ))?;
        write!(stdout, "●")?;

        stdout.flush()?;
        Ok(())
    }
}

fn draw_startup_screen<W: Write>(stdout: &mut W, width: u16, height: u16, elapsed: Duration) -> std::io::Result<()> {
    stdout.queue(crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
    
    let title = "🏓 PING PONG 🏓";
    let controls_w = "Controls: W/Up - Move Up";
    let controls_s = "S/Down - Move Down";
    let controls_esc = "ESC - Quit";
    let start_msg = "Game starts...";
    
    let title_y = height.saturating_sub(10);
    let controls_y = title_y + 2;
    
    // Center text
    stdout.queue(crossterm::cursor::MoveTo(
        width.saturating_sub(title.len() as u16) / 2,
        title_y,
    ))?;
    write!(stdout, "{}", title)?;
    
    stdout.queue(crossterm::cursor::MoveTo(
        width.saturating_sub(controls_w.len() as u16) / 2,
        controls_y,
    ))?;
    write!(stdout, "{}", controls_w)?;
    
    stdout.queue(crossterm::cursor::MoveTo(
        width.saturating_sub(controls_s.len() as u16) / 2,
        controls_y + 1,
    ))?;
    write!(stdout, "{}", controls_s)?;
    
    stdout.queue(crossterm::cursor::MoveTo(
        width.saturating_sub(controls_esc.len() as u16) / 2,
        controls_y + 2,
    ))?;
    write!(stdout, "{}", controls_esc)?;
    
    stdout.queue(crossterm::cursor::MoveTo(
        width.saturating_sub(start_msg.len() as u16) / 2,
        controls_y + 4,
    ))?;
    write!(stdout, "{}", start_msg)?;
    
    // Show progress indicator
    let progress = (elapsed.as_secs_f32() / STARTUP_DURATION.as_secs_f32() * 20.0) as usize;
    let progress_bar = format!("[{}{}]", "=".repeat(progress.min(20)), ".".repeat((20 - progress).min(20)));
    stdout.queue(crossterm::cursor::MoveTo(
        width.saturating_sub(progress_bar.len() as u16) / 2,
        controls_y + 5,
    ))?;
    write!(stdout, "{}", progress_bar)?;
    
    stdout.flush()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let (width, height) = crossterm::terminal::size()?;
    
    // Show startup screen for 2 seconds
    let startup_start = Instant::now();
    loop {
        let elapsed = startup_start.elapsed();
        draw_startup_screen(&mut stdout, width, height, elapsed)?;
        
        if elapsed >= STARTUP_DURATION {
            break;
        }
        
        // Handle skip with ESC during startup
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Esc || 
                   (key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('c')) {
                    execute!(stdout, LeaveAlternateScreen, Show)?;
                    disable_raw_mode()?;
                    return Ok(());
                }
            }
        }
    }
    
    let mut game = Game::new(width, height);
    let mut last_tick = Instant::now();

    // FPS tracking
    let mut frame_count = 0u32;
    let mut fps_timer = Instant::now();

    'gameloop: loop {
        let timeout = FRAME_TIME
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('c')
                {
                    break 'gameloop;
                }

                match key_event.code {
                    // Left paddle (Player) – supports W, S, and arrow keys for smooth movement
                    KeyCode::Char('w') | KeyCode::Up => {
                        game.paddle_a = (game.paddle_a - 2.0)
                            .max(PADDLE_HEIGHT as f32 / 2.0)
                    }
                    KeyCode::Char('s') | KeyCode::Down => {
                        game.paddle_a = (game.paddle_a + 2.0)
                            .min(game.height as f32 - PADDLE_HEIGHT as f32 / 2.0)
                    }
                    KeyCode::Esc => break 'gameloop,
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= FRAME_TIME {
            game.update();
            game.draw(&mut stdout)?;
            last_tick = Instant::now();

            // FPS counter update every second
            frame_count += 1;
            if fps_timer.elapsed() >= Duration::from_secs(1) {
                game.fps = frame_count;
                frame_count = 0;
                fps_timer = Instant::now();
            }

            // Win condition – exit game loop
            if game.game_over {
                break 'gameloop;
            }
        }
    }

    // If game ended, show winner and wait for keypress
    if game.game_over {
        // Clear screen and show message centered
        execute!(
            stdout,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(width / 2 - 10, height / 2)
        )?;
        if let Some(winner) = &game.winner {
            write!(stdout, "{} wins! Press any key to exit.", winner)?;
        }
        stdout.flush()?;

        // Wait for any key
        loop {
            if event::poll(Duration::from_secs(1))? {
                let _ = event::read()?;
                break;
            }
        }
    }

    execute!(stdout, LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;
    Ok(())
}
