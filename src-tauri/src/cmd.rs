use crate::config::get;
use crate::config::StoreWrapper;
use crate::error::Error;
use crate::SelectionInfoWrapper;
use crate::StringWrapper;
use crate::APP;
use log::{error, info};
use serde_json::{json, Value};
use std::io::Read;
use tauri::Manager;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, KEYBD_EVENT_FLAGS,
    VK_CONTROL, VK_V, VK_DELETE,
};
use windows::Win32::UI::WindowsAndMessaging::{
     SetForegroundWindow, GetForegroundWindow,
    EnumWindows, 
    BringWindowToTop, IsWindow,
};
use arboard::Clipboard;
use std::time::{Duration, Instant};

#[tauri::command]
pub fn get_text(state: tauri::State<StringWrapper>) -> String {
    info!("Get text: {:?}", state.0.lock().unwrap());
    return state.0.lock().unwrap().to_string();
}

#[tauri::command]
pub fn reload_store() {
    let state = APP.get().unwrap().state::<StoreWrapper>();
    let mut store = state.0.lock().unwrap();
    store.load().unwrap();
}

#[tauri::command]
pub fn cut_image(left: u32, top: u32, width: u32, height: u32, app_handle: tauri::AppHandle) {
    use dirs::cache_dir;
    use image::GenericImage;
    info!("Cut image: {}x{}+{}+{}", width, height, left, top);
    let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
    app_cache_dir_path.push(&app_handle.config().tauri.bundle.identifier);
    app_cache_dir_path.push("pot_screenshot.png");
    if !app_cache_dir_path.exists() {
        return;
    }
    let mut img = match image::open(&app_cache_dir_path) {
        Ok(v) => v,
        Err(e) => {
            error!("{:?}", e.to_string());
            return;
        }
    };
    let img2 = img.sub_image(left, top, width, height);
    app_cache_dir_path.pop();
    app_cache_dir_path.push("pot_screenshot_cut.png");
    match img2.to_image().save(&app_cache_dir_path) {
        Ok(_) => {}
        Err(e) => {
            error!("{:?}", e.to_string());
        }
    }
}

#[tauri::command]
pub fn get_base64(app_handle: tauri::AppHandle) -> String {
    use base64::{engine::general_purpose, Engine as _};
    use dirs::cache_dir;
    use std::fs::File;
    use std::io::Read;
    let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
    app_cache_dir_path.push(&app_handle.config().tauri.bundle.identifier);
    app_cache_dir_path.push("pot_screenshot_cut.png");
    if !app_cache_dir_path.exists() {
        return "".to_string();
    }
    let mut file = File::open(app_cache_dir_path).unwrap();
    let mut vec = Vec::new();
    match file.read_to_end(&mut vec) {
        Ok(_) => {}
        Err(e) => {
            error!("{:?}", e.to_string());
            return "".to_string();
        }
    }
    let base64 = general_purpose::STANDARD.encode(&vec);
    base64.replace("\r\n", "")
}

#[tauri::command]
pub fn copy_img(app_handle: tauri::AppHandle, width: usize, height: usize) -> Result<(), Error> {
    use arboard::{Clipboard, ImageData};
    use dirs::cache_dir;
    use image::ImageReader;
    use std::borrow::Cow;

    let mut app_cache_dir_path = cache_dir().expect("Get Cache Dir Failed");
    app_cache_dir_path.push(&app_handle.config().tauri.bundle.identifier);
    app_cache_dir_path.push("pot_screenshot_cut.png");
    let data = ImageReader::open(app_cache_dir_path)?.decode()?;

    let img = ImageData {
        width,
        height,
        bytes: Cow::from(data.as_bytes()),
    };
    let result = Clipboard::new()?.set_image(img)?;
    Ok(result)
}

#[tauri::command]
pub fn set_proxy() -> Result<bool, ()> {
    let host = match get("proxy_host") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => return Err(()),
    };
    let port = match get("proxy_port") {
        Some(v) => v.as_i64().unwrap(),
        None => return Err(()),
    };
    let no_proxy = match get("no_proxy") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => return Err(()),
    };
    let proxy = format!("http://{}:{}", host, port);

    std::env::set_var("http_proxy", &proxy);
    std::env::set_var("https_proxy", &proxy);
    std::env::set_var("all_proxy", &proxy);
    std::env::set_var("no_proxy", &no_proxy);
    Ok(true)
}

#[tauri::command]
pub fn unset_proxy() -> Result<bool, ()> {
    std::env::remove_var("http_proxy");
    std::env::remove_var("https_proxy");
    std::env::remove_var("all_proxy");
    std::env::remove_var("no_proxy");
    Ok(true)
}

#[tauri::command]
pub fn install_plugin(path_list: Vec<String>) -> Result<i32, Error> {
    let mut success_count = 0;

    for path in path_list {
        if !path.ends_with("potext") {
            continue;
        }
        let path = std::path::Path::new(&path);
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let file_name = file_name.replace(".potext", "");
        if !file_name.starts_with("plugin") {
            return Err(Error::Error(
                "Invalid Plugin: file name must start with plugin".into(),
            ));
        }

        let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        #[allow(unused_mut)]
        let mut plugin_type: String;
        if let Ok(mut info) = zip.by_name("info.json") {
            let mut content = String::new();
            info.read_to_string(&mut content)?;
            let json: serde_json::Value = serde_json::from_str(&content)?;
            plugin_type = json["plugin_type"]
                .as_str()
                .ok_or(Error::Error("can't find plugin type in info.json".into()))?
                .to_string();
        } else {
            return Err(Error::Error("Invalid Plugin: miss info.json".into()));
        }
        if zip.by_name("main.js").is_err() {
            return Err(Error::Error("Invalid Plugin: miss main.js".into()));
        }
        let config_path = dirs::config_dir().unwrap();
        let config_path =
            config_path.join(APP.get().unwrap().config().tauri.bundle.identifier.clone());
        let config_path = config_path.join("plugins");
        let config_path = config_path.join(plugin_type);
        let plugin_path = config_path.join(file_name);
        std::fs::create_dir_all(&config_path)?;
        zip.extract(&plugin_path)?;

        success_count += 1;
    }
    Ok(success_count)
}

#[tauri::command]
pub fn run_binary(
    plugin_type: String,
    plugin_name: String,
    cmd_name: String,
    args: Vec<String>,
) -> Result<Value, Error> {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let config_path = dirs::config_dir().unwrap();
    let config_path = config_path.join(APP.get().unwrap().config().tauri.bundle.identifier.clone());
    let config_path = config_path.join("plugins");
    let config_path = config_path.join(plugin_type);
    let plugin_path = config_path.join(plugin_name);

    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("cmd");
    #[cfg(target_os = "windows")]
    let cmd = cmd.creation_flags(0x08000000);
    #[cfg(target_os = "windows")]
    let cmd = cmd.args(["/c", &cmd_name]);
    #[cfg(not(target_os = "windows"))]
    let mut cmd = Command::new(&cmd_name);

    let output = cmd.args(args).current_dir(plugin_path).output()?;
    Ok(json!({
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        "status": output.status.code().unwrap_or(-1),
    }))
}

#[tauri::command]
pub fn font_list() -> Result<Vec<String>, Error> {
    use font_kit::source::SystemSource;
    let source = SystemSource::new();

    Ok(source.all_families()?)
}

#[tauri::command]
pub fn open_devtools(window: tauri::Window) {
    use tauri::Manager;
    
    if let Some(devtools) = window.get_window("__tauri_devtools") {
        devtools.close().unwrap();
    } else {
        #[cfg(debug_assertions)]
        window.open_devtools();
    }
}

#[tauri::command]
pub fn is_devtools_open(window: tauri::Window) -> bool {
    use tauri::Manager;
    window.get_window("__tauri_devtools").is_some()
}

#[tauri::command]
pub fn replace_selected_text(new_text: String) -> Result<(), String> {
    info!("Replace selected text: {}", new_text);

    let app_handle = APP.get().unwrap();
    info!("Got app handle");

    let selection_state = app_handle.state::<SelectionInfoWrapper>();
    info!("Got selection state");

    let selection_info = selection_state.0.lock().unwrap();
    info!("Got selection info lock");

    let info = match &*selection_info {
        Some(info) => {
            info!("Found selection info: {}", new_text);
            info
        }
        None => {
            info!("No selection info available");
            return Err("No selection info available".to_string());
        }
    };

    #[cfg(target_os = "windows")]
    {
        info!("Replace selected windows");
        
        info!("Looking for window: {}", info.window_id);
        
        let mut target_data: (Option<HWND>, String) = (None, info.window_id.clone());
        unsafe {
            info!("Starting window enumeration");
            let result = EnumWindows(
                Some(enum_window_callback), 
                windows::Win32::Foundation::LPARAM(&mut target_data as *mut _ as isize)
            );
            info!("EnumWindows completed with result: {}", result.is_ok());
            
            if target_data.0.is_none() {
                info!("No matching window found after enumeration");
            }
        }
        
        if let Some(hwnd) = target_data.0 {
            info!("Found window with HWND: {:?}", hwnd);
            
            unsafe {
                // 验证窗口是否有效
                if !IsWindow(hwnd).as_bool() {
                    error!("Invalid window handle");
                    return Err("Window is no longer valid".to_string());
                }
                info!("Window handle is valid");

                // 尝试设置剪贴板
                info!("Setting clipboard content");
                match Clipboard::new().and_then(|mut cb| cb.set_text(&new_text)) {
                    Ok(_) => info!("Clipboard set successfully"),
                    Err(e) => {
                        error!("Failed to set clipboard: {}", e);
                        return Err("Failed to set clipboard".to_string());
                    }
                }

                // 尝试激活窗口
                info!("Attempting to activate window");
                let bring_result = BringWindowToTop(hwnd);
                info!("BringWindowToTop result: {}", bring_result.is_ok());
                
                if !SetForegroundWindow(hwnd).as_bool() {
                    error!("Failed to set foreground window");
                    return Err("Could not activate window".to_string());
                }
                info!("SetForegroundWindow succeeded");

                // 等待窗口真正激活
                let start = Instant::now();
                let timeout = Duration::from_secs(2);
                let mut activated = false;
                while GetForegroundWindow() != hwnd {
                    if start.elapsed() > timeout {
                        error!("Timeout waiting for window activation");
                        return Err("Window activation timeout".to_string());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                    activated = true;
                }
                if activated {
                    info!("Window activated after waiting");
                } else {
                    info!("Window was already active");
                }

                // 等待确保窗口完全响应
                std::thread::sleep(Duration::from_millis(300));

                // 发送Delete键
                info!("Sending Delete key");
                let mut delete_input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_DELETE,
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                let delete_result = SendInput(&[delete_input], std::mem::size_of::<INPUT>() as i32);
                info!("Delete key down result: {}", delete_result);
                
                delete_input.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
                let delete_up_result = SendInput(&[delete_input], std::mem::size_of::<INPUT>() as i32);
                info!("Delete key up result: {}", delete_up_result);

                std::thread::sleep(Duration::from_millis(100));

                // 发送Ctrl+V
                info!("Sending Ctrl+V");
                let mut inputs = [
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VK_CONTROL,
                                wScan: 0,
                                dwFlags: KEYBD_EVENT_FLAGS(0),
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    },
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VK_V,
                                wScan: 0,
                                dwFlags: KEYBD_EVENT_FLAGS(0),
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    },
                ];
                
                let ctrl_v_result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                info!("Ctrl+V down result: {}", ctrl_v_result);
                
                std::thread::sleep(Duration::from_millis(50));
                
                // 释放按键
                inputs[0].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
                inputs[1].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
                let ctrl_v_up_result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                info!("Ctrl+V up result: {}", ctrl_v_up_result);
            }
            info!("Text replacement completed");
            return Ok(());
        } else {
            error!("Window not found: {}", info.window_id);
            return Err("Window not found".to_string());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        return Err("Platform not supported".to_string());
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthA, GetWindowTextA};
    use windows::Win32::Foundation::BOOL;
    use log::{info};
    
    let target_data = lparam.0 as *mut (Option<HWND>, String);
    let target_data = &mut *target_data;
    let (target_hwnd, target_title) = target_data;
    
    // 获取窗口标题长度
    let len = GetWindowTextLengthA(hwnd);
    if len > 0 {
        // 获取窗口标题
        let mut buffer = vec![0u8; (len + 1) as usize];
        let copied = GetWindowTextA(hwnd, &mut buffer);
        if copied > 0 {
            let window_title = String::from_utf8_lossy(&buffer[..copied as usize]).to_string();
            let window_title = window_title.trim();
            info!("Checking window: '{}'", window_title);
            
            // 提取固定部分（去掉最后的数字）
            let window_base = window_title.split(" – ").next().unwrap_or(window_title);
            let target_base = target_title.split(" – ").next().unwrap_or(&target_title);
            
            // 移除所有不可见字符
            let window_base = window_base.chars().filter(|c| !c.is_whitespace()).collect::<String>();
            let target_base = target_base.chars().filter(|c| !c.is_whitespace()).collect::<String>();
            
            info!("Comparing base titles: '{}' vs '{}'", window_base, target_base);
            
            // 检查窗口标题的固定部分是否匹配
            if window_base == target_base {
                info!("Found matching window: '{}'", window_title);
                *target_hwnd = Some(hwnd);
                return BOOL(0); // 停止枚举
            }
        }
    }
    BOOL(1) // 继续枚举
}
