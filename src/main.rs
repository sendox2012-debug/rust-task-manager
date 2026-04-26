use std::io::{self, Write, Read};
use std::process::Command;
use std::fmt::Write as FmtWrite;
use std::time::Duration;

const RESET: &str = "\x1B[0m";
const HIDE_CURSOR: &str = "\x1B[?25l";
const SHOW_CURSOR: &str = "\x1B[?25h";

#[derive(Clone)]
struct Task {
    name: String,
    url: Option<String>,
    done: bool,
}

enum AppState {
    Menu,
    Dashboard,
}

fn main() {
    let mut tasks: Vec<Task> = Vec::new();
    let mut cursor: usize = 0;
    let mut state = AppState::Menu;
    
    let mut stdout = io::stdout();
    let mut stdin = io::stdin();

    let is_raw = enable_raw_mode().is_ok();
    let _ = stdout.write_all(HIDE_CURSOR.as_bytes());
    let _ = stdout.flush();

    loop {
        clear_screen();
        
        match state {
            AppState::Menu => draw_menu(),
            AppState::Dashboard => draw_dashboard(&tasks, cursor),
        }

        let mut buf = [0u8; 1];
        if stdin.read_exact(&mut buf).is_err() { break; }

        match state {
            AppState::Menu => {
                match buf[0] {
                    b'1' | b' ' | 10 | 13 => state = AppState::Dashboard,
                    b'q' | b'Q' | 27 => break,
                    _ => {}
                }
            },
            AppState::Dashboard => {
                // Стрелки
                if buf[0] == 27 {
                    if stdin.read_exact(&mut buf).is_ok() && buf[0] == 91 {
                        if stdin.read_exact(&mut buf).is_ok() {
                            match buf[0] {
                                65 => { if cursor > 0 { cursor -= 1; } }
                                66 => { if !tasks.is_empty() && cursor < tasks.len() - 1 { cursor += 1; } }
                                _ => {}
                            }
                        }
                    }
                    continue;
                }

                match buf[0] {
                    b'w' | b'W' => { if cursor > 0 { cursor -= 1; } },
                    b's' | b'S' => { if !tasks.is_empty() && cursor < tasks.len() - 1 { cursor += 1; } },
                    
                    b' ' | 10 | 13 => { 
                        if !tasks.is_empty() { tasks[cursor].done = !tasks[cursor].done; } 
                    },
                    
                    b'a' | b'A' => {
                        // 1. Показываем курсор и выключаем raw-режим для ввода
                        let _ = stdout.write_all(SHOW_CURSOR.as_bytes());
                        let _ = stdout.flush();
                        if is_raw { disable_raw_mode().ok(); }

                        println!(); // Отступ
                        
                        // 2. Ввод НАЗВАНИЯ
                        print!("   >>> Название задачи: ");
                        io::stdout().flush().unwrap();
                        let mut name_input = String::new();
                        io::stdin().read_line(&mut name_input).unwrap_or_default();
                        let name = name_input.trim().to_string();

                        if !name.is_empty() {
                            // 3. Ввод ССЫЛКИ (только если название не пустое)
                            print!("   >>> Ссылка (Enter если нет): ");
                            io::stdout().flush().unwrap();
                            let mut url_input = String::new();
                            io::stdin().read_line(&mut url_input).unwrap_or_default();
                            let url_val = url_input.trim().to_string();

                            tasks.push(Task { 
                                name, 
                                url: if url_val.is_empty() { None } else { Some(url_val) }, 
                                done: false 
                            });
                            if cursor >= tasks.len() { cursor = tasks.len() - 1; }
                        }

                        // 4. Возвращаем настройки терминала
                        if is_raw {
                            enable_raw_mode().ok();
                            let _ = stdout.write_all(HIDE_CURSOR.as_bytes());
                        } else {
                            let _ = stdout.write_all(HIDE_CURSOR.as_bytes());
                        }
                        let _ = stdout.flush();
                    },

                    b'd' | b'D' => {
                        if !tasks.is_empty() {
                            tasks.remove(cursor);
                            if cursor >= tasks.len() && cursor > 0 { cursor -= 1; }
                        }
                    },
                    b'g' | b'G' => {
                        if !tasks.is_empty() {
                            if let Some(ref url) = tasks[cursor].url { 
                                open_url(url); 
                                std::thread::sleep(Duration::from_millis(500));
                            }
                        }
                    },
                    b'm' | b'M' | 27 => state = AppState::Menu,
                    b'q' | b'Q' => break,
                    _ => {}
                }
            }
        }
    }

    let _ = stdout.write_all(SHOW_CURSOR.as_bytes());
    let _ = stdout.flush();
    if is_raw { disable_raw_mode().ok(); }
    println!("\nПока!\n");
}

fn clear_screen() { print!("\x1B[2J\x1B[H"); }

fn draw_menu() {
    let title = rgb_text(" RUST TASK MANAGER ", 0, 255, 255);
    println!("\n\n");
    println!("   {}", title);
    println!("\n\n");
    println!("   {}", rgb_text("1. Начать работу", 100, 255, 100));
    println!("   {}", rgb_text("Q. Выход", 255, 100, 100));
    println!("\n\n   Нажмите '1' для старта...");
}

fn draw_dashboard(tasks: &[Task], cursor: usize) {
    let width = 70;
    let header = rgb_text(" DASHBOARD ", 0, 255, 255);
    
    println!("╔{}╗", rgb_text(&"═".repeat(width), 100, 100, 100));
    print!("║");
    let pad = (width - 11) / 2;
    println!("{:pad$}{}{:pad$}║", "", header, "", pad = pad);
    println!("╚{}╝\n", rgb_text(&"═".repeat(width), 100, 100, 100));

    if tasks.is_empty() {
        println!("   {}", rgb_text("Список пуст. Нажми 'A', чтобы добавить задачу.", 150, 150, 150));
    } else {
        for (i, t) in tasks.iter().enumerate() {
            let sel = i == cursor;
            let mark = if t.done { rgb_text("[✓]", 0, 255, 100) } else { rgb_text("[ ]", 255, 100, 100) };
            let ptr = if sel { rgb_text("►", 255, 255, 0) } else { " ".to_string() };
            let bg = if sel { "\x1B[48;2;30;30;50m" } else { "" };
            let r_bg = if sel { RESET } else { "" };
            
            let name = if t.done { format!("\x1B[9m{}\x1B[29m", t.name) } else { t.name.clone() };
            
            print!("   {}{} {} {}{}", ptr, bg, mark, name, r_bg);
            if let Some(_) = t.url {
                let link_lbl = if sel { "[G]" } else { "[link]" };
                print!(" {}", rgb_text(link_lbl, 100, 200, 255));
            }
            println!();
        }
    }

    println!("\n{}", rgb_text(&"─".repeat(width), 50, 50, 50));
    println!("   {}", rgb_text("УПРАВЛЕНИЕ:", 255, 255, 255));
    println!("   {} W / S          - Выбор задачи", rgb_text("•", 200, 200, 200));
    println!("   {} Пробел         - Отметить выполнено", rgb_text("•", 200, 200, 200));
    println!("   {} A              - Добавить задачу", rgb_text("•", 100, 255, 100));
    println!("   {} D              - Удалить задачу", rgb_text("•", 255, 100, 100));
    println!("   {} G              - Открыть ссылку", rgb_text("•", 100, 200, 255));
    println!("   {} M / Esc        - Главное меню", rgb_text("•", 255, 200, 100));
    println!("   {} Q              - Выход", rgb_text("•", 255, 100, 100));
}

fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = Command::new("xdg-open").arg(url).spawn();
}

fn enable_raw_mode() -> Result<(), String> {
    Command::new("stty").args(["raw", "-echo", "min", "1", "time", "0"]).status().map_err(|e| e.to_string())?;
    Ok(())
}

fn disable_raw_mode() -> Result<(), String> {
    Command::new("stty").args(["-raw", "echo", "icanon"]).status().map_err(|e| e.to_string())?;
    Ok(())
}

fn rgb_text(text: &str, r: u8, g: u8, b: u8) -> String {
    let mut res = String::new();
    let len = text.len() as f32;
    if len == 0.0 { return res; }
    for (i, c) in text.chars().enumerate() {
        let ratio = i as f32 / len;
        let nr = (r as f32 + 50.0 * ratio) as u8;
        let ng = (g as f32 + 50.0 * ratio) as u8;
        let nb = (b as f32 + 100.0 * ratio) as u8;
        let _ = FmtWrite::write_str(&mut res, &format!("\x1B[38;2;{};{};{}m{}", nr, ng, nb, c));
    }
    res.push_str(RESET);
    res
}