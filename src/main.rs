use std::error::Error;
use std::{io, thread};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use crossterm::{
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyCode},
    ExecutableCommand,
    terminal,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use rusty_audio::Audio;
use invaders::{frame::{Drawable}, render, frame};
use invaders::frame::Frame;
use invaders::invaders::Invaders;
use invaders::player::Player;

fn render_screen(render_rx: Receiver<Frame>) {
    let mut last_frame = frame::new_frame();
    let mut stdout = io::stdout();
    render::render(&mut stdout, &last_frame, &last_frame, true);
    while let Ok(curr_frame) = render_rx.recv() {
        render::render(&mut stdout, &last_frame, &curr_frame, false);
        last_frame = curr_frame;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = Audio::new();
    for item in &["explode", "lose", "move", "pew", "startup", "win"] {
        audio.add(item, &format!("sounds/{}.wav", item));
    }

    audio.play("startup");

    // Terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;

    // Render loop in a seperate thread
    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = thread::spawn(move || {
        render_screen(render_rx)
    });

    // Init game player & instant
    let mut player = Player::new();
    let mut instant = Instant::now();
    let mut invaders = Invaders::new();

    // Game loop
    'gameloop: loop {
        // Per-frame init
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut curr_frame = frame::new_frame();

        // Input
        while poll(Duration::default())? {
            if let Event::Key(key_event) = read()? {
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("lose");
                        break 'gameloop;
                    }
                    KeyCode::Left => player.move_left(),
                    KeyCode::Right => player.move_right(),
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        if player.shoot() {
                            audio.play("pew");
                        }
                    }
                    _ => {}
                }
            }
        }

        // Update timers
        player.update(delta);
        if invaders.update(delta) {
            audio.play("move");
        }

        // Detect shots
        if player.detect_hits(&mut invaders) {
            audio.play("explode");
        }

        // Draw & Render - silent ignore result
        // player.draw(&mut curr_frame);
        // invaders.draw(&mut curr_frame);

        let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];
        for drawable in drawables {
            drawable.draw(&mut curr_frame);
        }
        let _ = render_tx.send(curr_frame);
        thread::sleep(Duration::from_millis(1));

        // Check win or lose
        if invaders.all_killed() {
            audio.play("win");
            break 'gameloop;
        } else if invaders.reached_bottom() {
            audio.play("lose");
            break 'gameloop;
        }
    }

    // Cleanup
    drop(render_tx);
    render_handle.join().unwrap();
    audio.wait();

    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
