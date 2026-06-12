#[derive(serde::Serialize)]
struct ProductInfo {
    name: &'static str,
    version: &'static str,
}

#[tauri::command]
fn product_info() -> ProductInfo {
    ProductInfo {
        name: attestrum_transcription_core::PRODUCT_NAME,
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![product_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
