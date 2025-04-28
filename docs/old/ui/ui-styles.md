# Gini Terminal UI Style Guide

**Author:** kunihir0  
**Email:** kunihir0@example.com  
**Date:** 2025-04-26  
**Version:** 1.0

## Document Information

| | |
|---|---|
| Date/Time (UTC) | 2025-04-26 21:37:53 |
| Author | kunihir0 |

## Introduction

This document defines the visual design language and interaction patterns for Gini's terminal-based interfaces. It provides guidelines for implementing both the Command Line Interface (CLI) and Text-based User Interface (TUI), ensuring a cohesive, aesthetically pleasing experience while maintaining consistency with the plugin architecture.

The design principles prioritize:

* Visual delight without sacrificing functionality
* Consistent color and symbol conventions
* Informative progress indicators
* Appropriate feedback for both interactive and non-interactive modes
* Accessibility through thoughtful color and contrast choices

## Visual Design System

### Color Palette

#### Primary Colors (ANSI Escape Codes)

| Color Name | ANSI Code | Usage Context |
|---|---|---|
| Pink | `\033[38;5;219m` | Primary branding, key highlights, primary actions |
| Purple | `\033[38;5;183m` | Secondary elements, gradient components, plugin boundaries |
| Cyan | `\033[38;5;123m` | Interactive elements, information indicators, VM status |
| Yellow | `\033[38;5;228m` | Warnings, attention highlights, configuration notices |
| Green | `\033[38;5;156m` | Success messages, completion indicators, healthy status |
| Red | `\033[38;5;210m` | Critical actions, error states, destructive operations |
| Blue | `\033[38;5;111m` | Calm, non-threatening information, passive elements |

#### Secondary Colors

| Color Name | ANSI Code | Usage Context |
|---|---|---|
| Magenta | `\033[38;5;201m` | Special highlights, emphasis, OSX-specific features |
| Light Blue | `\033[38;5;159m` | Subtle information, background details, help text |
| Lavender | `\033[38;5;147m` | Gentle prompts, soft interactions, passive states |
| Peach | `\033[38;5;223m` | Soft warnings, tertiary highlights, minor notices |
| Mint | `\033[38;5;121m` | Secondary success indicators, resource optimization |

#### Background Colors

| Background | ANSI Code | Usage Context |
|---|---|---|
| Black background | `\033[40m` | Default background |
| Dark background | `\033[48;5;236m` | Dialog backgrounds, modal windows |
| Purple background | `\033[45m` | Highlight areas, selection indicators |
| Pink background | `\033[48;5;219m` | Current action indicator, critical notices |

#### Text Styling

| Style | ANSI Code | Effect |
|---|---|---|
| Bold | `\033[1m` | Emphasize important information, commands, options |
| Italic | `\033[3m` | Descriptions, quotes, supplementary information |
| Underline | `\033[4m` | Hyperlinks, navigation options, selectable items |
| Blink | `\033[5m` | Critical warnings only (use sparingly) |

### Typography & Symbols

#### Icon System

##### Status Symbols

| Status | Symbol | Color Coding |
|---|---|---|
| Success | `✓` | green |
| Warning | `!` | yellow |
| Error | `✗` | red |
| Info | `✧` | cyan |
| Progress | `→` | blue |
| Star | `★` | purple |
| VM | `◈` | pink |
| Plugin | `⚙` | cyan |

##### Animation Character Sets

| Spinner Type | Characters | Usage Context |
|---|---|---|
| Flower Spinner | `✿`, `❀`, `✾`, `❁`, `✽`, `✼`, `✻`, `✺`, `✹`, `✸` | General purpose, default style |
| Star Spinner | `✦`, `✧`, `✩`, `✪`, `✫`, `✬`, `✭`, `✮` | VM preparation, OpenCore operations |
| Braille Spinner | `⠋`, `⠙`, `⠹`, `⠸`, `⠼`, `⠴`, `⠦`, `⠧`, `⠇`, `⠏` | Technical operations, filesystem tasks |
| Arrows Spinner | `←`, `↖`, `↑`, `↗`, `→`, `↘`, `↓`, `↙` | Network operations, data transfers |
| Pulse Spinner | `•`, `○`, `●`, `○` | Resource monitoring, status checks |
| Bounce Spinner | `⠁`, `⠂`, `⠄`, `⡀`, `⢀`, `⠠`, `⠐`, `⠈` | Recovery operations, diagnostic tools |

#### Border & Frame Styles

| Frame Style | Characters | Usage Context |
|---|---|---|
| Single Frame | `╭─────╮`<br>`│     │`<br>`╰─────╯` | General dialogs, default style |
| Double Frame | `╔═════╗`<br>`║     ║`<br>`╚═════╝` | Important notices, critical information |
| Bold Frame | `┏━━━━━┓`<br>`┃     ┃`<br>`┗━━━━━┛` | Configuration screens, settings |
| Dotted Frame | `.....`<br>`.   .`<br>`.....` | Optional information, tips |
| ASCII Frame | `+-----+`<br>`\|     \|`<br>`+-----+` | Fallback for limited terminals |
| Stars Frame | `✦✧✧✧✧✦`<br>`✧   ✧`<br>`✦✧✧✧✧✦` | Special announcements, achievements |

## Animation Guidelines

### CLI Animation Principles

The Command Line Interface should incorporate animations that provide feedback without interfering with information processing:

1. **Progressive Disclosure**: Start with minimal animations for critical information, escalate visual richness for success/completion states
2. **Unobtrusive Progress**: Animations should indicate progress without dominating screen space
3. **Meaningful Motion**: Each animation should convey specific information (not merely decorative)
4. **Speed Consideration**: Animation timing should reflect actual progress, not arbitrary durations
5. **Fallback Options**: All animations must have non-animated alternatives for accessibility and CI/CD environments

### TUI Animation Principles

The Text User Interface should leverage animations to enhance navigation and provide context:

1. **State Transitions**: Use animations to indicate movement between screens/states
2. **Focus Indicators**: Subtle animations should highlight the current focus point
3. **Background Activity**: Use ambient animations to indicate ongoing background processes
4. **Consistent Language**: Animation patterns should be consistent across similar operations
5. **Performance Impact**: Animations must not cause noticeable performance degradation

### Standard Animation Techniques

#### Text Effects

##### Gradient Text
Used for headings and important titles to create visual interest:

```rust
fn gradient_text(text: &str, colors: &[&str]) -> String {
    let mut result = String::new();
    for (i, char) in text.chars().enumerate() {
        let color_idx = ((i as f32 / text.chars().count() as f32) * colors.len() as f32) as usize;
        result.push_str(&format!("{}{}{}", 
            COLORS[colors[color_idx.min(colors.len() - 1)]], 
            char, 
            COLORS["reset"]));
    }
    result
}
```

##### Typing Effect
Used for introductory text and important notices:

```rust
fn typing_effect(text: &str, speed: f32) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    
    for char in text.chars() {
        // Calculate realistic typing delay with variance
        let variance = 0.3;
        let delay = speed * (1.0 + rand::random::<f32>() * variance - variance/2.0);
        
        // Extra delay for punctuation
        let delay = if ".!?,;:".contains(char) {
            delay * 2.0
        } else {
            delay
        };
        
        write!(lock, "{}", char).unwrap();
        lock.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_secs_f32(delay));
    }
}
```

##### Wave Text
Used for celebratory messages and success indicators:

```rust
fn wave_text(text: &str, cycles: usize, amplitude: usize) {
    let term_size = terminal_size();
    let width = term_size.0;
    let mut oscillator = 0.0;
    
    for _ in 0..cycles {
        for step in 0..20 {
            oscillator = std::f32::consts::PI * 2.0 * (step as f32 / 20.0);
            
            // Clear line
            print!("\r{}", " ".repeat(width as usize));
            
            // Print each character with vertical offset
            for (i, ch) in text.chars().enumerate() {
                let char_oscillator = oscillator + (i as f32 * 0.2);
                let offset = (amplitude as f32 * char_oscillator.sin()) as isize;
                
                if offset >= 0 {
                    // Move down and print
                    print!("\x1B[{}B{}\x1B[{}A", offset, ch, offset);
                } else {
                    // Move up and print
                    print!("\x1B[{}A{}\x1B[{}B", offset.abs(), ch, offset.abs());
                }
            }
            
            std::io::stdout().flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}
```

#### Progress Indicators

##### Dynamic Progress Bar
Used for long-running tasks with known progress:

```rust
fn progress_bar(
    progress: f32, 
    width: usize, 
    text: &str,
    pulse: bool
) {
    let actual_progress = progress.min(1.0).max(0.0);
    
    // Apply pulse effect if requested
    let effective_progress = if pulse {
        let pulse_amount = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as f32 / 500.0)
            .sin()
            .abs() * 0.05;
            
        actual_progress * (0.95 + pulse_amount)
    } else {
        actual_progress
    };
    
    let filled = (width as f32 * effective_progress) as usize;
    let empty = width - filled;
    
    print!("\r{} [{}{}] {:>5.1}% ", 
        text,
        COLORS["cyan"].to_string() + &"●".repeat(filled) + COLORS["reset"],
        "○".repeat(empty),
        actual_progress * 100.0
    );
    std::io::stdout().flush().unwrap();
}
```

##### Multi-Style Spinner
Used for operations with unknown duration:

```rust
fn spinner(text: &str, spinner_type: &str, duration_secs: f32) {
    let frames = match spinner_type {
        "flower" => vec!["✿", "❀", "✾", "❁", "✽", "✼", "✻", "✺", "✹", "✸"],
        "star" => vec!["✦", "✧", "✩", "✪", "✫", "✬", "✭", "✮"],
        "braille" => vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        "arrows" => vec!["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
        "pulse" => vec!["•", "○", "●", "○"],
        _ => vec!["◐", "◓", "◑", "◒"],
    };
    
    let start = std::time::Instant::now();
    let mut frame_idx = 0;
    
    while start.elapsed().as_secs_f32() < duration_secs {
        print!("\r{} {} ", 
            COLORS["pink"].to_string() + frames[frame_idx] + COLORS["reset"],
            text
        );
        std::io::stdout().flush().unwrap();
        
        frame_idx = (frame_idx + 1) % frames.len();
        std::thread::sleep(std::time::Duration::from_millis(80));
    }
    print!("\r{}\r", " ".repeat(text.len() + 3));
}
```

##### Step Indicator
Used for multi-stage processes:

```rust
fn print_step(step: usize, total_steps: usize, message: &str, status: &str) {
    let status_symbol = match status {
        "success" => format!("{} ✓ ", COLORS["green"]),
        "error" => format!("{} ✗ ", COLORS["red"]),
        "warning" => format!("{} ! ", COLORS["yellow"]),
        "progress" => format!("{} → ", COLORS["blue"]),
        _ => format!("{} • ", COLORS["cyan"]),
    };
    
    let progress = format!("[{}/{}]", step, total_steps);
    
    println!("{}{}{} {} {}{}", 
        status_symbol, 
        COLORS["reset"],
        COLORS["purple"],
        progress,
        COLORS["reset"],
        message
    );
}
```

#### Scene Transitions

##### Fade Transition
Used when switching between major interface sections:

```rust
fn fade_transition() {
    let term_size = terminal_size();
    let width = term_size.0 as usize;
    let height = term_size.1 as usize;
    
    // Save cursor position
    print!("\x1B[s");
    
    // Characters for gradient effect
    let fade_chars = [" ", "░", "▒", "▓", "█"];
    
    // Fade out
    for &ch in fade_chars.iter().rev() {
        for y in 0..height {
            print!("\x1B[{};1H", y+1);
            print!("{}", ch.repeat(width));
        }
        std::io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(80));
    }
    
    // Clear screen
    print!("\x1B[2J\x1B[1;1H");
    
    // Fade in
    for &ch in fade_chars.iter() {
        for y in 0..height {
            print!("\x1B[{};1H", y+1);
            print!("{}", ch.repeat(width));
        }
        std::io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(80));
    }
    
    // Restore cursor
    print!("\x1B[u");
    std::io::stdout().flush().unwrap();
}
```

##### Sparkle Effect
Used for completion states and achievements:

```rust
fn sparkle_effect(text: &str, duration_secs: f32) {
    let term_size = terminal_size();
    let width = term_size.0 as usize;
    let mid_y = term_size.1 as usize / 2;
    
    let sparkles = ["✨", "✧", "✦", "⋆", "✩", "✫", "✬"];
    let colors = ["pink", "purple", "cyan", "yellow", "blue"];
    
    let text_start = (width - text.len()) / 2;
    let start_time = std::time::Instant::now();
    
    while start_time.elapsed().as_secs_f32() < duration_secs {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        
        // Print centered text
        print!("\x1B[{};{}H{}", mid_y, text_start, text);
        
        // Add random sparkles around the text
        for _ in 0..10 {
            let x = rand::random::<usize>() % width;
            let y_offset = (rand::random::<usize>() % 5) - 2;
            let y = (mid_y as isize + y_offset as isize).max(1) as usize;
            
            let sparkle = sparkles[rand::random::<usize>() % sparkles.len()];
            let color = colors[rand::random::<usize>() % colors.len()];
            
            print!("\x1B[{};{}H{}{}{}", 
                y, 
                x, 
                COLORS[color], 
                sparkle,
                COLORS["reset"]
            );
        }
        
        std::io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
```

## CLI Interface Guidelines

### Command Structure

#### Naming Conventions

* Use kebab-case for command and flag names (`create-vm` not `createVM` or `create_vm`)
* Prefix destructive commands with verbs (`remove-vm` not just `vm-remove`)
* Use consistent terminology across similar operations

#### Command Flow

```
gini <command> [subcommand] [options]
```

Standard command categories:

| Category | Description |
|---|---|
| create | Creation commands (VMs, configurations, resources) |
| list | Display available resources (VMs, plugins, configs) |
| manage | Management operations (start, stop, modify) |
| config | Configuration commands |
| plugin | Plugin management |
| test | Testing operations |

#### Flag Consistency

* Short flags: Single hyphen with single character (`-v`)
* Long flags: Double hyphen with full name (`--version`)
* Boolean flags don't take values (`--dry-run` not `--dry-run=true`)
* Value flags use equals or space (`--name="My VM"` or `--name "My VM"`)

### CLI Animation Usage

#### Basic Operations

For simple, quick operations:

```rust
fn simple_operation(message: &str) {
    // Display a simple spinner during the operation
    spinner(message, "braille", 1.5);
    println!("{} {}", 
        COLORS["green"] + "✓" + COLORS["reset"],
        message + " completed"
    );
}
```

#### Multi-stage Operations

For complex operations with multiple steps:

```rust
fn multi_stage_operation(operation_name: &str, steps: &[&str]) {
    println!("{}", gradient_text(&format!("• {} •", operation_name), 
                             &["purple", "pink", "cyan"]));
    println!();
    
    for (i, step) in steps.iter().enumerate() {
        // Show spinner with current step
        spinner(step, "flower", 0.8 + (i as f32 * 0.1));
        
        // Print completed step
        print_step(i+1, steps.len(), step, "success");
    }
    
    // Add sparkle effect at the end
    sparkle_effect(&format!("✨ {} Completed! ✨", operation_name), 1.2);
}
```

#### Dry Run Mode

Dry run mode should use the same visual style but clearly indicate simulation:

```rust
fn dry_run_operation(operation_name: &str, steps: &[&str]) {
    let dry_run_header = format!("[DRY RUN] {}", operation_name);
    
    // Special frame for dry run
    print_boxed_text(&dry_run_header, "single", "yellow");
    println!();
    
    for (i, step) in steps.iter().enumerate() {
        // Print each step with "would" prefix
        print_step(i+1, steps.len(), &format!("Would {}", step), "info");
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    
    println!("\n{} This was a dry run. No changes were made.", 
        COLORS["yellow"] + "!" + COLORS["reset"]);
}
```

#### Progress Displays

For operations with measurable progress:

```rust
fn progress_operation(operation_name: &str, total_steps: usize) {
    println!("{}", operation_name);
    
    for i in 0..=total_steps {
        let progress = i as f32 / total_steps as f32;
        
        // Show pulse effect in the last 10%
        let pulse = progress > 0.9;
        
        progress_bar(
            progress,
            30,
            &format!("Step {}/{}", i, total_steps),
            pulse
        );
        
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    println!("\n{} Operation complete", 
        COLORS["green"] + "✓" + COLORS["reset"]);
}
```

### CLI Examples

#### VM Creation Example

```
$ gini create-vm --name "Monterey Dev" --os-version monterey --ram 8G

✨ Creating macOS VM: Monterey Dev ✨

[1/7] ✓ Validating configuration
[2/7] ✓ Preparing storage volumes
[3/7] ✓ Downloading OpenCore components
[4/7] ✓ Building EFI configuration
[5/7] ✓ Adding recovery image
[6/7] ✓ Generating VM definition
[7/7] ✓ Creating startup script

✨ VM Creation Complete! ✨

Your VM is ready to use. Start it with:
  gini start-vm "Monterey Dev"
```

#### Dry Run Example

```
$ gini create-vm --name "Monterey Dev" --os-version monterey --dry-run

╭───────────────────────────────────────╮
│ [DRY RUN] Creating VM: Monterey Dev   │
╰───────────────────────────────────────╯

[1/7] • Would validate configuration
[2/7] • Would prepare storage (50GB required)
[3/7] • Would download OpenCore v0.8.5 (15MB)
[4/7] • Would build EFI configuration
[5/7] • Would download recovery image (629MB)
[6/7] • Would generate VM definition
[7/7] • Would create startup script

! This was a dry run. No changes were made.
! Total estimated disk usage: 694MB
```

#### Error State Example

```
$ gini create-vm --name "Monterey Dev" --ram 128G

✨ Creating macOS VM: Monterey Dev ✨

[1/3] ✓ Validating configuration
[2/3] ✗ Preparing storage volumes

Error: Insufficient free space
Required: 50GB
Available: 23GB

Suggestions:
• Free up disk space
• Specify a smaller disk with --disk-size
• Use a different storage location with --storage-path
```

## TUI Interface Guidelines

### Layout Structure

#### Screen Regions

```
┌─────────────────────────────────────────────────┐
│                    Header                       │
├─────────────────────────────────────────────────┤
│                                                 │
│                                                 │
│                  Main Content                   │
│                                                 │
│                                                 │
├──────────────────────┬──────────────────────────┤
│    Status Region     │     Controls/Help        │
└──────────────────────┴──────────────────────────┘
```

#### Navigation System

* Tab navigation between major sections
* Arrow keys for movement within sections
* Enter to select/activate
* Escape to go back/cancel
* Consistent shortcut keys across screens

### TUI Animation Usage

#### Navigation Transitions

When moving between screens:

```rust
fn screen_transition(from_screen: &str, to_screen: &str) {
    // Save current screen state
    app.save_screen_state(from_screen);
    
    // Simple fade transition
    fade_transition();
    
    // Load new screen
    app.load_screen(to_screen);
    
    // Typing effect for screen title
    typing_effect(&format!("• {} •", to_screen.to_uppercase()), 0.02);
}
```

#### Background Processing

For operations running while UI remains interactive:

```rust
fn background_task_indicator(task_name: &str, is_active: bool) {
    if is_active {
        // Show subtle spinner in status bar
        let spinner_frames = ["•", "○", "●", "○"];
        let frame = spinner_frames[app.animation_frame % spinner_frames.len()];
        
        status_bar.set_left_text(
            &format!("{} {} {}", 
                COLORS["blue"],
                frame,
                COLORS["reset"] + task_name
            )
        );
    } else {
        status_bar.set_left_text("");
    }
}
```

#### Ambient Animation

Subtle background animations for visual interest:

```rust
fn update_ambient_animations() {
    // Only update every few frames for performance
    if app.frame_count % 5 != 0 {
        return;
    }
    
    // Subtle particle effect in empty areas
    if app.settings.enable_ambient_effects {
        for particle in &mut app.ambient_particles {
            // Update position with slight drift
            particle.x += particle.dx;
            particle.y += particle.dy;
            
            // Wrap around screen
            if particle.x < 0.0 { particle.x = app.width as f32; }
            if particle.x > app.width as f32 { particle.x = 0.0; }
            if particle.y < 0.0 { particle.y = app.height as f32; }
            if particle.y > app.height as f32 { particle.y = 0.0; }
            
            // Draw particle if in empty space
            let x = particle.x as usize;
            let y = particle.y as usize;
            if app.is_empty_space(x, y) {
                app.canvas.put_char(
                    x, y, 
                    particle.char,
                    COLORS[particle.color]
                );
            }
        }
    }
}
```

### TUI Components 

#### VM List Component

```
┌─ Virtual Machines ───────────────────────────────┐
│ • Monterey Dev                      [Running ✓]  │
│ • Big Sur Test                      [Stopped ○]  │
│ • Ventura Build Server              [Paused ⏸]   │
│ • Catalina Legacy                   [Stopped ○]  │
└─────────────────────────────────────────────────┘
```

```rust
fn render_vm_list(vms: &[VirtualMachine], selected_idx: usize) {
    let status_symbols = [
        ("Running", "✓", "green"),
        ("Stopped", "○", "blue"),
        ("Paused", "⏸", "yellow"),
        ("Error", "✗", "red"),
    ];
    
    // Frame header with gradient text
    print_boxed_header("Virtual Machines", "single", &["pink", "purple"]);
    
    for (i, vm) in vms.iter().enumerate() {
        // Find status symbol and color
        let (_, symbol, color) = status_symbols
            .iter()
            .find(|(status, _, _)| status == &vm.status)
            .unwrap_or(&("Unknown", "?", "red"));
            
        // Highlight selected VM
        let prefix = if i == selected_idx { 
            COLORS["pink"] + "• " + COLORS["reset"]
        } else {
            "  ".to_string()
        };
        
        // VM name with status
        println!("{}{}{:30} [{} {}{}]",
            prefix,
            if i == selected_idx { COLORS["bold"] } else { "" },
            vm.name,
            vm.status,
            COLORS[color] + symbol + COLORS["reset"],
            if i == selected_idx { COLORS["reset"] } else { "" }
        );
    }
    
    print_boxed_footer("single");
}
```

#### Dialog Box Component

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃               Confirm Action                    ┃
┃                                                 ┃
┃  Are you sure you want to delete this VM?       ┃
┃  This action cannot be undone.                  ┃
┃                                                 ┃
┃          [Cancel]         [Delete]              ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

```rust
fn show_dialog(title: &str, message: &str, options: &[&str], dangerous: bool) -> usize {
    // Choose frame style based on dialog type
    let frame_style = if dangerous { "bold" } else { "single" };
    let title_colors = if dangerous { 
        vec!["red", "orange"] 
    } else { 
        vec!["cyan", "blue"] 
    };
    
    // Calculate dialog size
    let width = 50;
    let height = 6 + message.lines().count();
    
    // Position at center of screen
    let term_size = terminal_size();
    let start_x = (term_size.0 as usize - width) / 2;
    let start_y = (term_size.1 as usize - height) / 2;
    
    // Save screen content behind dialog
    let saved_area = save_screen_area(start_x, start_y, width, height);
    
    // Draw dialog with slight fade-in
    fade_in_dialog(start_x, start_y, width, height);
    
    // Draw frame and title
    draw_frame(start_x, start_y, width, height, frame_style);
    print_centered_text(start_y + 1, &gradient_text(title, &title_colors));
    
    // Print message
    for (i, line) in message.lines().enumerate() {
        print_centered_text(start_y + 3 + i, line);
    }
    
    // Draw buttons
    let selected = draw_dialog_buttons(start_x, start_y, width, height, options);
    
    // Restore screen when done
    restore_screen_area(saved_area, start_x, start_y, width, height);
    
    selected
}
```

#### Resource Monitor Component

```
┌─ VM Resources ──────────────────────────────────┐
│ CPU: ████████████████████░░░░░░░░░░  67%        │
│ RAM: █████████████████████████████░  93%        │
│ DSK: ██████░░░░░░░░░░░░░░░░░░░░░░░  24%        │
│ NET: ███░░░░░░░░░░░░░░░░░░░░░░░░░░  12% ↑ 2MB/s│
└─────────────────────────────────────────────────┘
```

```rust
fn render_resource_monitor(vm_stats: &VmStats) {
    // Frame with title
    print_boxed_header("VM Resources", "single", &["cyan", "blue"]);
    
    // CPU bar
    let cpu_percentage = (vm_stats.cpu_usage * 100.0) as usize;
    print!("CPU: ");
    render_bar(30, vm_stats.cpu_usage, "green", "cpu_usage > 0.8");
    println!(" {:3}%", cpu_percentage);
    
    // RAM bar
    let ram_percentage = (vm_stats.ram_usage * 100.0) as usize;
    print!("RAM: ");
    render_bar(30, vm_stats.ram_usage, "purple", "ram_usage > 0.9");
    println!(" {:3}%", ram_percentage);
    
    // Disk bar
    let disk_percentage = (vm_stats.disk_usage * 100.0) as usize;
    print!("DSK: ");
    render_bar(30, vm_stats.disk_usage, "blue", "false");
    println!(" {:3}%", disk_percentage);
    
    // Network with transfer rate
    let net_percentage = (vm_stats.net_usage * 100.0) as usize;
    print!("NET: ");
    render_bar(30, vm_stats.net_usage, "cyan", "false");
    println!(" {:3}% {} {}/s", 
        net_percentage,
        if vm_stats.net_tx > vm_stats.net_rx { "↑" } else { "↓" },
        format_bytes(vm_stats.net_tx.max(vm_stats.net_rx))
    );
    
    print_boxed_footer("single");
}

fn render_bar(width: usize, fill: f32, color: &str, alert_condition: &str) {
    let filled_width = (width as f32 * fill) as usize;
    let empty_width = width - filled_width;
    
    // Evaluate alert condition
    let is_alert = match alert_condition {
        "cpu_usage > 0.8" => fill > 0.8,
        "ram_usage > 0.9" => fill > 0.9,
        _ => false
    };
    
    // Choose color based on alert state
    let bar_color = if is_alert { "red" } else { color };
    
    print!("{}{}{}", 
        COLORS[bar_color],
        "█".repeat(filled_width),
        COLORS["reset"] + &"░".repeat(empty_width)
    );
}
```

### TUI Example Pages

#### Main Dashboard

```
┌─ Gini ─────────────────────────────────────┐
│                                                 │
│ ✨ Welcome to Gini VM Manager ✨           │
│                                                 │
├─ Virtual Machines ───────────────────────────────┤
│ • Monterey Dev                      [Running ✓]  │
│ • Big Sur Test                      [Stopped ○]  │
│ • Ventura Build Server              [Paused ⏸]   │
│                                                 │
├─ Quick Actions ────────────────────────────────┤
│  [Create VM]    [Start VM]    [Settings]        │
│                                                 │
├─ System Status ─────────────────────────────────┤
│ Storage: 234GB free                             │
│ Plugins: 12 loaded                              │
│                                                 │
└─────────────────────────────────────────────────┘
 [F1] Help   [F5] Refresh   [F10] Quit
```

#### VM Creation Wizard

```
┌─ Create VM: Step 2/4 ─────────────────────────────┐
│                                                   │
│  Hardware Configuration                           │
│                                                   │
│  ✿ CPU: [_____4_____] cores                       │
│                                                   │
│  ✿ RAM: [____8192____] MB                         │
│                                                   │
│  ✿ Disk: [____50_____] GB                         │
│                                                   │
│  ✿ Graphics: [VFIO Passthrough  ▼]                │
│                                                   │
│  ✿ Network: [Bridged Adapter    ▼]                │
│                                                   │
└───────────────────────────────────────────────────┘
 [⬅ Back]                                  [Next ➡]
```

## Accessibility Considerations

### Terminal Compatibility

* All visual elements must have fallback options for limited terminals
* Support for both 256-color and basic 16-color terminals
* ASCII alternatives for Unicode characters
* Non-animated alternatives for all animations

### Color Blindness

* Color is never the sole indicator of status
* All color-based information is supplemented with symbols
* Test color schemes with color blindness simulators
* Maintain sufficient contrast ratios for all text

### Performance Modes

* Provide a "low animation" mode for slow terminals or remote connections
* Add a "high contrast" mode for visibility-focused display
* Include a "CI/CD" mode with no animations and minimal formatting

## Implementation Guidelines

### Animation Framework

```rust
// Animation trait for standardizing animation interfaces
pub trait Animation {
    fn update(&mut self, delta_time: f32) -> bool;
    fn render(&self);
    fn reset(&mut self);
    fn is_complete(&self) -> bool;
}

// Animation manager for controlling multiple animations
pub struct AnimationManager {
    animations: HashMap<String, Box<dyn Animation>>,
    global_scale: f32,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            animations: HashMap::new(),
            global_scale: 1.0,
        }
    }
    
    pub fn add(&mut self, name: &str, animation: Box<dyn Animation>) {
        self.animations.insert(name.to_string(), animation);
    }
    
    pub fn update(&mut self, delta_time: f32) {
        let scaled_time = delta_time * self.global_scale;
        let mut completed = Vec::new();
        
        for (name, animation) in &mut self.animations {
            if animation.update(scaled_time) {
                completed.push(name.clone());
            }
        }
        
        for name in completed {
            self.animations.remove(&name);
        }
    }
    
    pub fn render(&self) {
        for (_, animation) in &self.animations {
            animation.render();
        }
    }
    
    pub fn set_speed_scale(&mut self, scale: f32) {
        self.global_scale = scale;
    }
}
```

### Component System

```rust
// UI component trait
pub trait UiComponent {
    fn render(&self);
    fn handle_input(&mut self, input: Input) -> Option<UiAction>;
    fn get_bounds(&self) -> Rect;
    fn set_bounds(&mut self, bounds: Rect);
    fn is_focused(&self) -> bool;
    fn set_focused(&mut self, focused: bool);
}

// Standard frame component
pub struct Frame {
    bounds: Rect,
    title: String,
    style: FrameStyle,
    focused: bool,
    title_colors: Vec<String>,
    content: Vec<Box<dyn UiComponent>>,
}

impl Frame {
    pub fn new(title: &str, style: FrameStyle) -> Self {
        Self {
            bounds: Rect::new(0, 0, 40, 10),
            title: title.to_string(),
            style,
            focused: false,
            title_colors: vec!["cyan".to_string()],
            content: Vec::new(),
        }
    }
    
    pub fn with_gradient_title(mut self, colors: Vec<&str>) -> Self {
        self.title_colors = colors.iter().map(|&s| s.to_string()).collect();
        self
    }
    
    pub fn add_component(&mut self, component: Box<dyn UiComponent>) {
        self.content.push(component);
    }
}

impl UiComponent for Frame {
    fn render(&self) {
        let (x, y, width, height) = self.bounds.into();
        
        // Draw frame
        draw_frame(x, y, width, height, &self.style);
        
        // Draw title with gradient
        if !self.title.is_empty() {
            let title = if self.title_colors.len() > 1 {
                gradient_text(&self.title, &self.title_colors.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            } else {
                COLORS[&self.title_colors[0]] + &self.title + COLORS["reset"]
            };
            
            print_centered_text_at(x + width / 2, y, &title);
        }
        
        // Draw child components
        for component in &self.content {
            component.render();
        }
    }
    
    // Other trait method implementations
    fn handle_input(&mut self, input: Input) -> Option<UiAction> {
        // Handle input and route to focused child component
        None
    }
    
    fn get_bounds(&self) -> Rect {
        self.bounds
    }
    
    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }
    
    fn is_focused(&self) -> bool {
        self.focused
    }
    
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
```

## Implementation Plan

### Phase 1: Core UI Components

* Implement basic text styling and color system
* Create core UI components (frames, text blocks, progress bars)
* Establish animation framework foundation
* Build basic input handling

### Phase 2: CLI Interface

* Implement complete CLI command structure
* Add CLI-specific animations and progress indicators
* Create command documentation system
* Build error handling and display

### Phase 3: TUI Framework

* Develop TUI layout system
* Build navigation and focus management
* Implement interactive components
* Create screen transition system

### Phase 4: Animation Enhancements

* Add advanced text effects
* Implement particles and ambient animations
* Create sophisticated progress indicators
* Build transition effects

### Phase 5: Polish and Accessibility

* Implement terminal compatibility detection
* Add accessibility modes
* Create configuration system for UI preferences
* Performance optimization for slow terminals

## Appendix: Terminal Support

| Terminal | Color Support | Unicode Support | Animation Support |
|---|---|---|---|
| iTerm2 | Full (24-bit) | Excellent | Excellent |
| GNOME Terminal | Full (24-bit) | Excellent | Very Good |
| Konsole | Full (24-bit) | Excellent | Very Good |
| Windows Terminal | Full (24-bit) | Very Good | Very Good |
| Alacritty | Full (24-bit) | Excellent | Excellent |
| Terminal.app | Limited (256-color) | Good | Fair |
| PuTTY | Limited (256-color) | Limited | Fair |
| SSH Connections | Varies | Varies | Limited |

## Appendix: Ansi Color Reference

```rust
// Color codes map
pub static COLORS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "reset" => "\x1B[0m",
    "bold" => "\x1B[1m",
    "italic" => "\x1B[3m",
    "underline" => "\x1B[4m",
    "blink" => "\x1B[5m",
    "pink" => "\x1B[38;5;219m",
    "purple" => "\x1B[38;5;183m",
    "cyan" => "\x1B[38;5;123m",
    "yellow" => "\x1B[38;5;228m",
    "blue" => "\x1B[38;5;111m",
    "orange" => "\x1B[38;5;216m",
    "green" => "\x1B[38;5;156m",
    "red" => "\x1B[38;5;210m",
    "magenta" => "\x1B[38;5;201m",
    "light_blue" => "\x1B[38;5;159m",
    "lavender" => "\x1B[38;5;147m",
    "peach" => "\x1B[38;5;223m",
    "mint" => "\x1B[38;5;121m",
};