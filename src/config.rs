#[derive(Debug)]
pub struct Config {
    pub download_path: &'static str,
    pub audio_quality: AudioQuality,
    pub save_cover: bool,
    pub cover_size: CoverSize,
    pub exist_check: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            download_path: "./",
            audio_quality: AudioQuality::High,
            save_cover: true,
            cover_size: CoverSize::Big,
            exist_check: true,
        }
    }
}

#[derive(Debug)]
enum AudioQuality {
    Normal,
    High,
    Master,
}

#[derive(Debug)]
enum CoverSize {
    Small,
    Normal,
    Big,
}

// Download path, Audio quality, Save covers
