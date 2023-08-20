#[derive(Debug)]
pub struct Config {
    pub download_path: String,
    pub audio_quality: AudioQuality,
    pub save_cover: bool,
    pub exist_check: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            download_path: String::from("./"),
            audio_quality: AudioQuality::High,
            save_cover: true,
            exist_check: true,
        }
    }
}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
pub enum AudioQuality {
    Normal,
    High,
    Master,
}

impl std::str::FromStr for AudioQuality {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Normal" => Ok(AudioQuality::Normal),
            "High" => Ok(AudioQuality::High),
            "Master" => Ok(AudioQuality::Master),
            _ => Err(format!("invalid AudioQuality enum type: {}", s)),
        }
    }
}

// Download path, Audio quality, Save covers
