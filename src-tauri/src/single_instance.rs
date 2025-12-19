// Single-instance enforcement utilities
// Prevents multiple instances of the application from running simultaneously

use std::fs;
use std::io::Write;
use std::path::Path;
use tracing::{info, error, warn};

// Windows-specific imports for process checking
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
#[cfg(windows)]
use windows_sys::Win32::Foundation::CloseHandle;

/// Check if a Windows process is running using Win32 API (fast method, <1ms)
#[cfg(windows)]
fn is_process_running_windows(pid: u32) -> bool {
    use std::time::Instant;
    let start = Instant::now();
    
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        let elapsed = start.elapsed();
        
        if handle == 0 {
            info!("PID {} not running (Win32 check took {:?})", pid, elapsed);
            false
        } else {
            CloseHandle(handle);
            info!("PID {} is running (Win32 check took {:?})", pid, elapsed);
            true
        }
    }
}

/// Fallback process check using tasklist command (slow method, ~5000ms)
#[cfg(not(windows))]
fn is_process_running_fallback(pid_str: &str) -> bool {
    let output = std::process::Command::new("tasklist")
        .args(&["/FI", &format!("PID eq {}", pid_str), "/NH"])
        .output();
    
    match output {
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            output_str.contains(pid_str)
        }
        Err(e) => {
            error!("Failed to run tasklist command: {}", e);
            false
        }
    }
}

/// Clean up lockfile (explicit cleanup function)
pub fn cleanup_lockfile(app_data_dir: &Path) -> Result<(), String> {
    let lock_path = app_data_dir.join("course_scheduler.lock");
    info!("Cleaning up lockfile: {}", lock_path.display());
    
    if !lock_path.exists() {
        info!("Lockfile doesn't exist, nothing to clean up");
        return Ok(());
    }
    
    fs::remove_file(&lock_path)
        .map_err(|e| {
            error!("Failed to delete lockfile: {}", e);
            format!("Failed to delete lockfile: {}", e)
        })?;
    
    info!("Lockfile cleaned up successfully");
    Ok(())
}

/// Check for stale lockfile and clean it up if process is not running
/// Returns Ok(true) if stale lock was cleaned, Ok(false) if no lock, Err if lock is valid
pub fn check_stale_lockfile(app_data_dir: &Path) -> Result<bool, String> {
    let lock_path = app_data_dir.join("course_scheduler.lock");
    
    if !lock_path.exists() {
        info!("No lockfile found");
        return Ok(false);
    }
    
    info!("Found existing lockfile, checking if stale");
    
    // Read PID from lockfile
    let pid_str = match fs::read_to_string(&lock_path) {
        Ok(content) => content.trim().to_string(),
        Err(e) => {
            warn!("Failed to read lockfile: {}", e);
            // Can't read lockfile, assume it's stale and delete it
            info!("Deleting unreadable lockfile");
            let _ = fs::remove_file(&lock_path);
            return Ok(true); // Cleaned stale lock
        }
    };
    
    // Parse PID
    let pid: u32 = match pid_str.parse() {
        Ok(p) => p,
        Err(e) => {
            warn!("Invalid PID in lockfile: {}", e);
            info!("Deleting lockfile with invalid PID");
            let _ = fs::remove_file(&lock_path);
            return Ok(true); // Cleaned stale lock
        }
    };
    
    // Check if process is running using Win32 API (Windows) or fallback (non-Windows)
    #[cfg(windows)]
    let is_running = is_process_running_windows(pid);
    
    #[cfg(not(windows))]
    let is_running = is_process_running_fallback(&pid_str);
    
    if is_running {
        error!("Process {} is still running", pid);
        return Err(format!(
            "应用程序已在运行 (进程 ID: {})\n\n\
            如果应用程序已关闭但仍显示此消息，请手动删除锁文件：\n{}",
            pid,
            lock_path.display()
        ));
    } else {
        // Process not running, lockfile is stale
        warn!("Stale lockfile detected for PID {}", pid);
        info!("Deleting stale lockfile");
        
        match fs::remove_file(&lock_path) {
            Ok(_) => {
                info!("Stale lockfile deleted successfully");
                Ok(true) // Cleaned stale lock
            }
            Err(e) => {
                error!("Failed to delete stale lockfile: {}", e);
                Err(format!("Failed to delete stale lockfile: {}", e))
            }
        }
    }
}

/// Check if another instance of the application is already running
/// Returns Ok(()) if no other instance exists, Err with message if lock exists
pub fn check_single_instance(app_data_dir: &Path) -> Result<(), String> {
    let lock_path = app_data_dir.join("course_scheduler.lock");
    
    info!("Checking for single instance, lockfile: {}", lock_path.display());
    
    // Check if lock file exists
    if lock_path.exists() {
        warn!("Lockfile already exists");
        // Try to read the lock file to get process info
        match fs::read_to_string(&lock_path) {
            Ok(content) => {
                let pid_str = content.trim();
                error!("Another instance is running with PID: {}", pid_str);
                return Err(format!(
                    "应用程序已在运行 (进程 ID: {})\n\n\
                    如果应用程序已关闭但仍显示此消息，请手动删除锁文件：\n{}",
                    pid_str,
                    lock_path.display()
                ));
            }
            Err(e) => {
                error!("Lockfile exists but couldn't read it: {}", e);
                return Err(format!(
                    "应用程序可能已在运行\n\n\
                    如果应用程序已关闭但仍显示此消息，请手动删除锁文件：\n{}",
                    lock_path.display()
                ));
            }
        }
    }
    
    // Create lock file with current process ID
    let pid = std::process::id();
    info!("Creating lockfile with PID: {}", pid);
    
    let mut lock_file = fs::File::create(&lock_path)
        .map_err(|e| {
            error!("Failed to create lockfile: {}", e);
            format!("无法创建锁文件: {}", e)
        })?;
    
    write!(lock_file, "{}", pid)
        .map_err(|e| {
            error!("Failed to write to lockfile: {}", e);
            format!("无法写入锁文件: {}", e)
        })?;
    
    info!("Lockfile created successfully");
    Ok(())
}

/// Remove the lock file on clean shutdown
pub fn remove_lock_file(app_data_dir: &Path) {
    let lock_path = app_data_dir.join("course_scheduler.lock");
    info!("Attempting to remove lockfile: {}", lock_path.display());
    
    if lock_path.exists() {
        match fs::remove_file(&lock_path) {
            Ok(_) => info!("Lockfile removed successfully"),
            Err(e) => error!("Failed to remove lockfile: {}", e),
        }
    } else {
        info!("Lockfile doesn't exist, nothing to remove");
    }
}
