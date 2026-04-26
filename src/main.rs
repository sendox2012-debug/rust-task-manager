use std::io::{self, Write, Read};
use std::process::Command;
use std::fmt::Write as FmtWrite;
use std::time::Duration;
use std::fs::File;
use std::path::Path;

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
                    b'1' | 10 | 13 => {
                        state = AppState::Dashboard;
                        clear_input_buffer();
                        std::thread::sleep(Duration::from_millis(100));
                    },
                    b'q' | b'Q' | 27 => break,
                    _ => {}
                }
            },
            AppState::Dashboard => {
                if buf[0] == 27 { 
                    state = AppState::Menu;
                    clear_input_buffer();
                    std::thread::sleep(Duration::from_millis(100));
                    continue; 
                }

                match buf[0] {
                    b'w' | b'W' => { if cursor > 0 { cursor -= 1; } },
                    b's' | b'S' => { if !tasks.is_empty() && cursor < tasks.len() - 1 { cursor += 1; } },
                    
                    b'm' | b'M' => { 
                        if !tasks.is_empty() { 
                            tasks[cursor].done = !tasks[cursor].done; 
                        } 
                    },
                    
                    b'a' | b'A' => {
                        let _ = stdout.write_all(SHOW_CURSOR.as_bytes());
                        let _ = stdout.flush();
                        if is_raw { disable_raw_mode().ok(); }

                        println!();
                        print!("   >>> Название [ссылка] [notwork]: ");
                        io::stdout().flush().unwrap();
                        
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap_or_default();
                        let input = input.trim();

                        if !input.is_empty() {
                            let parts: Vec<&str> = input.split_whitespace().collect();
                            
                            if !parts.is_empty() {
                                let mut name_parts = Vec::new();
                                let mut url: Option<String> = None;
                                let mut done = false;

                                for part in parts {
                                    if part == "notwork" {
                                        done = true;
                                    } else if part.starts_with("http") || (part.contains('.') && part.len() > 3 && !part.contains(' ')) {
                                        url = Some(part.to_string());
                                    } else {
                                        name_parts.push(part);
                                    }
                                }

                                let name = name_parts.join(" ");

                                if !name.is_empty() {
                                    tasks.push(Task { name, url, done });
                                    if cursor >= tasks.len() { cursor = tasks.len() - 1; }
                                }
                            }
                        }

                        if is_raw {
                            enable_raw_mode().ok();
                            let _ = stdout.write_all(HIDE_CURSOR.as_bytes());
                        } else {
                            let _ = stdout.write_all(HIDE_CURSOR.as_bytes());
                        }
                        let _ = stdout.flush();
                        clear_input_buffer();
                    },

                    b'd' | b'D' => {
                        if !tasks.is_empty() {
                            tasks.remove(cursor);
                            if cursor >= tasks.len() && cursor > 0 { cursor -= 1; }
                        }
                    },

                    // G - Открыть ссылку задачи
                    b'g' | b'G' => {
                        if !tasks.is_empty() {
                            if let Some(ref url) = tasks[cursor].url {
                                open_url(url);
                                print!("\n   {} Открываю: {}", rgb_text("→", 100, 200, 255), url);
                                let _ = io::stdout().flush();
                                std::thread::sleep(Duration::from_millis(1000));
                            } else {
                                print!("\n   {} У задачи нет ссылки", rgb_text("!", 255, 200, 100));
                                let _ = io::stdout().flush();
                                std::thread::sleep(Duration::from_millis(1000));
                            }
                        }
                    },

                    // E - Экспорт в TXT файл + открытие файла
                    b'e' | b'E' => {
                        let filename = "rust-todo-list.txt";
                        export_tasks_to_file(&tasks, filename);
                        open_file(filename);
                        std::thread::sleep(Duration::from_millis(1500));
                    },

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

// Функция экспорта задач в TXT файл
fn export_tasks_to_file(tasks: &[Task], filename: &str) {
    match File::create(filename) {
        Ok(mut file) => {
            let _ = writeln!(file, "════════════════════════════════════════");
            let _ = writeln!(file, "       RUST TODO-LIST by Sendo");
            let _ = writeln!(file, "════════════════════════════════════════");
            let _ = writeln!(file, "Дата: {}", chrono_lite_date());
            let _ = writeln!(file, "Всего задач: {}", tasks.len());
            let _ = writeln!(file, "────────────────────────────────────────");
            let _ = writeln!(file, "");

            if tasks.is_empty() {
                let _ = writeln!(file, "Список задач пуст.");
            } else {
                for (i, task) in tasks.iter().enumerate() {
                    let status = if task.done { "[ВЫПОЛНЕНО]" } else { "[В ОЖИДАНИИ]" };
                    let _ = writeln!(file, "{}. {} {}", i + 1, status, task.name);
                    
                    if let Some(ref url) = task.url {
                        let _ = writeln!(file, "   Link: {}", url);
                    }
                    let _ = writeln!(file, "");
                }
            }

            let completed = tasks.iter().filter(|t| t.done).count();
            let pending = tasks.len() - completed;
            let _ = writeln!(file, "────────────────────────────────────────");
            let _ = writeln!(file, "Выполнено: {}/{}", completed, tasks.len());
            let _ = writeln!(file, "В ожидании: {}", pending);
            let _ = writeln!(file, "════════════════════════════════════════");

            print!("\n   {} Файл '{}' сохранен!", rgb_text("OK", 0, 255, 100), filename);
            let _ = io::stdout().flush();
        }
        Err(e) => {
            print!("\n   {} Ошибка: {}", rgb_text("ERR", 255, 100, 100), e);
            let _ = io::stdout().flush();
        }
    }
}

// Открытие файла в системе
fn open_file(filename: &str) {
    if !Path::new(filename).exists() {
        return;
    }
    
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(&["/C", "start", "", filename])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open")
            .arg(filename)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open")
            .arg(filename)
            .spawn();
    }
}

// Открытие ссылки в браузере
fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(&["/C", "start", "", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open")
            .arg(url)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open")
            .arg(url)
            .spawn();
    }
}

fn chrono_lite_date() -> String {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell")
            .args(&["-Command", "Get-Date -Format 'dd.MM.yyyy HH:mm'"])
            .output();
        if let Ok(out) = output {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("date")
            .args(&["+%d.%m.%Y %H:%M"])
            .output();
        if let Ok(out) = output {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }
    "N/A".to_string()
}

fn clear_input_buffer() {
    std::thread::sleep(Duration::from_millis(50));
}

fn clear_screen() { print!("\x1B[2J\x1B[H"); }

fn draw_menu() {
    let art_lines = vec![
        "                                        ",
        "   ╔════════════════════════════════╗   ",
        "   ║                                ║   ",
        "   ║   📝 Rust Todo-list 📝         ║   ",
        "   ║                                ║   ",
        "   ║   by Sendo                     ║   ",
        "   ║   tg: @tg_sendo                ║   ",
        "   ║   ─────────────────────        ║   ",
        "   ║                                ║   ",
        "   ║   [1] Начать работу            ║   ",
        "   ║   [Q] Выход                    ║   ",
        "   ║                                ║   ",
        "   ╚════════════════════════════════╝   ",
        "                                        ",
        "                                        ",
    ];

    println!("\n\n");
    for line in art_lines {
        println!("   {}", rgb_text(line, 100, 200, 255));
    }
    println!("\n   Нажмите '1' для старта...");
}

fn draw_dashboard(tasks: &[Task], cursor: usize) {
    let width = 70;
    let header = rgb_text(" RUST TODO ", 0, 255, 255);
    
    println!("╔{}╗", rgb_text(&"═".repeat(width), 100, 100, 100));
    print!("║");
    let pad = (width - 11) / 2;
    println!("{:pad$}{}{:pad$}║", "", header, "", pad = pad);
    println!("╚{}╝\n", rgb_text(&"═".repeat(width), 100, 100, 100));

    if tasks.is_empty() {
        println!("   {}", rgb_text("Список пуст. Нажми 'A'.", 150, 150, 150));
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
    println!("   {}", rgb_text("КНОПКИ:", 255, 255, 255));
    println!("   {} W - Вверх", rgb_text("•", 200, 200, 200));
    println!("   {} S - Вниз", rgb_text("•", 200, 200, 200));
    println!("   {} M - Галочка (выполнено)", rgb_text("•", 200, 200, 200));
    println!("   {} A - Добавить ( [название (обяз.)] [ссылка: https://.. (не обяз)])", rgb_text("•", 100, 255, 100));
    println!("   {} D - Удалить", rgb_text("•", 255, 100, 100));
    println!("   {} G - Открыть ссылку задачи", rgb_text("•", 100, 200, 255));
    println!("   {} E - Экспорт в TXT + открыть", rgb_text("•", 200, 100, 255));
    println!("   {} Q - Выход", rgb_text("•", 255, 100, 100));
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