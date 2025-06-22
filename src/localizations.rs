use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

// Simple in-memory translations
struct Translations {
    strings: HashMap<&'static str, &'static str>,
}

impl Translations {
    fn new() -> Self {
        Self {
            strings: HashMap::new(),
        }
    }

    fn lookup(&self, key: &str) -> Option<&'static str> {
        self.strings.get(key).copied()
    }
}

pub struct Localizations {
    translations: HashMap<String, Translations>,
    current_lang: String,
}

impl Localizations {
    pub fn new() -> Self {
        let mut translations = HashMap::new();
        
        // English translations
        let mut en = Translations::new();
        en.strings.insert("app-title", "YouTube Downloader");
        en.strings.insert("url-label", "URL:");
        en.strings.insert("url-placeholder", "Enter video URL");
        en.strings.insert("download-button", "Download");
        en.strings.insert("download-format", "Download as:");
        en.strings.insert("format-mp4", "MP4 (Video)");
        en.strings.insert("format-mp3", "MP3 (Audio only)");
        en.strings.insert("status-ready", "Ready");
        en.strings.insert("status-downloading", "Downloading...");
        en.strings.insert("status-complete", "Download complete!");
        en.strings.insert("error-no-url", "Please enter a URL");
        en.strings.insert("error-ytdlp-missing", "yt-dlp not found. Please install it first.");
        translations.insert("en-US".to_string(), en);
        
        // Spanish translations
        let mut es = Translations::new();
        es.strings.insert("app-title", "Descargador de YouTube");
        es.strings.insert("url-label", "URL:");
        es.strings.insert("url-placeholder", "Ingresa la URL del video");
        es.strings.insert("download-button", "Descargar");
        es.strings.insert("download-format", "Descargar como:");
        es.strings.insert("format-mp4", "MP4 (Video)");
        es.strings.insert("format-mp3", "MP3 (Solo audio)");
        es.strings.insert("status-ready", "Listo");
        es.strings.insert("status-downloading", "Descargando...");
        es.strings.insert("status-complete", "¡Descarga completada!");
        es.strings.insert("error-no-url", "Por favor ingresa una URL");
        es.strings.insert("error-ytdlp-missing", "No se encontró yt-dlp. Por favor instálalo primero.");
        translations.insert("es-ES".to_string(), es);
        
        Self {
            translations,
            current_lang: "en-US".to_string(),
        }
    }
    
    pub fn language_loader(&self) -> &Self {
        self
    }
    
    pub fn lookup_single_language(&self, key: &str, _args: Option<&()>) -> Option<String> {
        self.translations
            .get(&self.current_lang)
            .and_then(|t| t.lookup(key))
            .or_else(|| {
                // Fallback to English if the current language doesn't have the key
                if self.current_lang != "en-US" {
                    self.translations.get("en-US").and_then(|t| t.lookup(key))
                } else {
                    None
                }
            })
            .map(String::from)
    }
    
    pub fn select(&mut self, lang: &LanguageIdentifier) -> Result<(), String> {
        // Try with full language-region code first
        let lang_region = format!(
            "{}-{}", 
            lang.language,
            lang.region.map(|r| r.to_string()).unwrap_or_default()
        );
        
        if self.translations.contains_key(&lang_region) {
            self.current_lang = lang_region;
            return Ok(());
        }
        
        // Try with just the language code
        let lang_only = lang.language.to_string();
        for (key, _) in &self.translations {
            if key.starts_with(&lang_only) {
                self.current_lang = key.clone();
                return Ok(());
            }
        }
        
        // Fallback to English if the requested language is not available
        self.current_lang = "en-US".to_string();
        Ok(())
    }
}
