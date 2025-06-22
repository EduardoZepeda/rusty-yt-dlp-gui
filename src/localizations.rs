use std::collections::HashMap;

// Simple in-memory translations
#[derive(Default)]
pub struct Translations {
    strings: HashMap<&'static str, &'static str>,
}

impl Translations {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn insert(&mut self, key: &'static str, value: &'static str) {
        self.strings.insert(key, value);
    }
    
    pub fn lookup(&self, key: &str) -> Option<&'static str> {
        self.strings.get(key).copied()
    }
}

pub struct Localizations {
    translations: HashMap<&'static str, Translations>,
    current_lang: String,
}

impl Localizations {
    pub fn new() -> Self {
        let mut translations = HashMap::new();
        
        // English translations
        let mut en = Translations::new();
        en.insert("app-title", "YouTube Downloader");
        en.insert("download-button", "Download");
        en.insert("update-button", "Update yt-dlp");
        en.insert("download-format", "Download as:");
        en.insert("format-mp4", "MP4 (Video)");
        en.insert("format-mp3", "MP3 (Audio only)");
        en.insert("url-label", "Video URL:");
        en.insert("url-placeholder", "Enter video URL");
        en.insert("status-ready", "Ready");
        en.insert("status-downloading", "Downloading:");
        en.insert("status-updating", "Updating yt-dlp...");
        en.insert("status-complete", "Download complete:");
        en.insert("error-invalid-url", "Error: Invalid URL");
        en.insert("error-ytdlp-not-found", "Error: yt-dlp not found. Please install yt-dlp and make sure it's in your PATH.");
        en.insert("update-success", "yt-dlp updated successfully");
        en.insert("update-failed", "Failed to update yt-dlp");
        translations.insert("en-US", en);
        
        // Spanish translations
        let mut es = Translations::new();
        es.insert("app-title", "Descargador de YouTube");
        es.insert("download-button", "Descargar");
        es.insert("update-button", "Actualizar yt-dlp");
        es.insert("download-format", "Descargar como:");
        es.insert("format-mp4", "MP4 (Video)");
        es.insert("format-mp3", "MP3 (Solo audio)");
        es.insert("url-label", "URL del video:");
        es.insert("url-placeholder", "Ingrese la URL del video");
        es.insert("status-ready", "Listo");
        es.insert("status-downloading", "Descargando:");
        es.insert("status-updating", "Actualizando yt-dlp...");
        es.insert("status-complete", "Descarga completada:");
        es.insert("error-invalid-url", "Error: URL inválida");
        es.insert("error-ytdlp-not-found", "Error: No se encontró yt-dlp. Por favor instale yt-dlp y asegúrese de que esté en su PATH.");
        es.insert("update-success", "yt-dlp actualizado correctamente");
        es.insert("update-failed", "Error al actualizar yt-dlp");
        translations.insert("es-ES", es);
        
        // Get system language
        let default_lang = if let Some(lang) = std::env::var("LANG").ok()
            .and_then(|l| l.split('_').next().map(|s| s.to_lowercase())) {
                if lang == "es" { "es-ES" } else { "en-US" }
            } else {
                "en-US"
            };
        
        let mut localizer = Self {
            translations,
            current_lang: default_lang.to_string(),
        };
        
        // Try to set the system language
        if let Ok(lang) = std::env::var("LANG") {
            if lang.starts_with("es") {
                let _ = localizer.select("es-ES");
            }
        }
        
        localizer
    }
    
    pub fn lookup_single_language(&self, key: &str, _args: Option<&()>) -> Option<String> {
        self.translations
            .get(self.current_lang.as_str())
            .and_then(|t| t.lookup(key))
            .map(|s| s.to_string())
            .or_else(|| {
                // Fallback to English if the current language doesn't have the key
                if self.current_lang != "en-US" {
                    self.translations.get("en-US").and_then(|t| t.lookup(key)).map(|s| s.to_string())
                } else {
                    None
                }
            })
    }
    

    
    pub fn select(&mut self, lang: &str) -> Result<(), String> {
        // Try exact match first
        if self.translations.contains_key(lang) {
            self.current_lang = lang.to_string();
            return Ok(());
        }
        
        // Try language code only
        let lang_part = lang.split('-').next().unwrap_or(lang);
        for &key in self.translations.keys() {
            if key.starts_with(lang_part) {
                self.current_lang = key.to_string();
                return Ok(());
            }
        }
        
        // Fallback to English
        self.current_lang = "en-US".to_string();
        Ok(())
    }
}
