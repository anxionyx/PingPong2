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
    paddle_a: i16, // Left paddle Y center
    paddle_b: i16, // Right paddle Y center
    score_a: u32,
    score_b: u32,
}

impl Game {
    fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            ball: Ball {
                x: (width / 2) as f32,
                y: (height / 2) as f32,
                dx: 0.5,
                dy: 0.2,
            },
            paddle_a: (height / 2) as i16,
            paddle_b: (height / 2) as i16,
            score_a: 0,
            score_b: 0,
        }
    }

    fn reset_ball(&mut self, direction: f32) {
        self.ball.x = (self.width / 2) as f32;
        self.ball.y = (self.height / 2) as f32;
        self.ball.dx = direction * 0.5;
        self.ball.dy = 0.2;
    }

    fn update(&mut self) {
        // Update ball position
        self.ball.x += self.ball.dx;
        self.ball.y += self.ball.dy;

        // Ceiling and floor collisions
        if self.ball.y <= 1.0 || self.ball.y >= (self.height - 1) as f32 {
            self.ball.dy = -self.ball.dy;
        }

        // Left Paddle Collision Mechanics
        if self.ball.x <= 3.0 && self.ball.x >= 2.0 {
            let paddle_top = self.paddle_a - PADDLE_HEIGHT / 2;
            let paddle_bottom = self.paddle_a + PADDLE_HEIGHT / 2;
            if self.ball.y >= paddle_top as f32 && self.ball.y <= paddle_bottom as f32 {
                self.ball.dx = -self.ball.dx * 1.05; // Slightly speed up on hit
                // Alter angle based on where it hit the paddle
                self.ball.dy += (self.ball.y - self.paddle_a as f32) * 0.1;
            }
        }

        // Right Paddle Collision Mechanics
        if self.ball.x >= (self.width - 3) as f32 && self.ball.x <= (self.width - 2) as f32 {
            let paddle_top = self.paddle_b - PADDLE_HEIGHT / 2;
            let paddle_bottom = self.paddle_b + PADDLE_HEIGHT / 2;
            if self.ball.y >= paddle_top as f32 && self.ball.y <= paddle_bottom as f32 {
                self.ball.dx = -self.ball.dx * 1.05;
                self.ball.dy += (self.ball.y - self.paddle_b as f32) * 0.1;
            }
        }

        // Score tracking
        if self.ball.x <= 0.0 {
            self.score_b += 1;
            self.reset_ball(1.0);
        } else if self.ball.x >= self.width as f32 {
            self.score_a += 1;
            self.reset_ball(-1.0);
        }
    }

    fn draw<W: Write>(&self, stdout: &mut W) -> std::ioResult<()> {
        // Clear screen using a single high-performance sweep
        stdout.queue(crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;

        // Render Scores
        stdout.queue(crossterm::cursor::MoveTo(self.width / 4, 1))?;
        print!("{}", self.score_a);
        stdout.queue(crossterm::cursor::MoveTo((self.width * 3) / 4, 1))?;
        print!("{}", self.score_b);

        // Render Center Line
        for y in 0..self.height {
            if y % 2 == 0 {
                stdout.queue(crossterm::cursor::MoveTo(self.width / 2, y))?;
                print!("|");
            }
        }

        // Render Left Paddle (Player A)
        let a_start = (self.paddle_a - PADDLE_HEIGHT / 2).max(0);
        for i in 0..PADDLE_HEIGHT {
            stdout.queue(crossterm::cursor::MoveTo(2, (a_start + i) as u16))?;
            print!("█");
        }

        // Render Right Paddle (Player B)
        let b_start = (self.paddle_b - PADDLE_HEIGHT / 2).max(0);
        for i in 0..PADDLE_HEIGHT {
            stdout.queue(crossterm::cursor::MoveTo(self.width - 3, (b_start + i) as u16))?;
            print!("█");
        }

        // Render Ball
        stdout.queue(crossterm::cursor::MoveTo(self.ball.x as u16, self.ball.y as u16))?;
        print!("●");

        stdout.flush()?;
        Ok(())
    }
}

fn main() -> std::ioResult<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let (width, height) = crossterm::terminal::size()?;
    let mut game = Game::new(width, height);
    let mut last_tick = Instant::now();

    'gameloop: loop {
        let timeout = FRAME_TIME
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key_event) = event::read()? {
                // Handle exits cleanly
                if key_event.modifiers.contains(KeyModifiers::CONTROL) && key_event.code == KeyCode::Char('c') {
                    break 'gameloop;
                }

                match key_event.code {
                    // Left Paddle Controls (W/S)
                    KeyCode::Char('w') => game.paddle_a = (game.paddle_a - 2).max(PADDLE_HEIGHT / 2),
                    KeyCode::Char('s') => game.paddle_a = (game.paddle_a + 2).min(game.height as i16 - PADDLE_HEIGHT / 2),
                    
                    // Right Paddle Controls (Up/Down Arrows)
                    KeyCode::Up => game.paddle_b = (game.paddle_b - 2).max(PADDLE_HEIGHT / 2),
                    KeyCode::Down => game.paddle_b = (game.paddle_b + 2).min(game.height as i16 - PADDLE_HEIGHT / 2),
                    
                    KeyCode::Esc => break 'gameloop,
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= FRAME_TIME {
            game.update();
            game.draw(&mut stdout)?;
            last_tick = Instant::now();
        }
    }

    // Clean cleanup to ensure the user's terminal isn't broken on exit
    execute!(stdout, LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;
    Ok(())
}
