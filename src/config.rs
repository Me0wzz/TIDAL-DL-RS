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
            audio_quality: AudioQuality::LOSSLESS,
            save_cover: true,
            exist_check: true,
        }
    }
}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
pub enum AudioQuality {
    LOW,
    HIGH,
    LOSSLESS,
    MASTER,
}

impl std::str::FromStr for AudioQuality {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LOW" => Ok(AudioQuality::LOW),
            "HIGH" => Ok(AudioQuality::HIGH),
            "LOSSLESS" => Ok(AudioQuality::LOSSLESS),
            "MASTER" => Ok(AudioQuality::MASTER),
            _ => Err(format!("invalid AudioQuality enum type: {}", s)),
        }
    }
}

// Download path, Audio quality, Save covers
