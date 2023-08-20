# TIDAL-DL-RS

---

Just working TIDAL Downloader written in Rust 

Now it's on development stage and contains unexpected bugs.

## Features
- [x] Concurrent Download
- [x] Download Music
- [x] Support ID3 Tags
- [x] Support Track, Album, Playlist URL
- [x] Configurable settings
- [x] (Partial) Support M4A/AAC (Metadata not supported yet)


## TODO
- [ ] Support 50 < files download (due to API limit)
- [ ] Refresh session token
- [ ] Improve code quality


## Usage
1. Clone this repository
2. `cargo run <URL1> <URL2> ...`

## Configurations

You can change settings by modifying `.tdlrs.json` located in `current directory`

### audio_quality

|  Value     |Quality|
|------------|-------|
|LOW, HIGH   |AAC    |
|LOSSLESS    |FLAC   |
|HI_RES      |MQA    |


### download_path
|  Value     |Result          |
|------------|----------------|
|String type   |custom download path|

### save_cover
|  Value     |Result          |
|------------|----------------|
|true   |Save cover.jpg       |
|false  |Don't save cover.jpg |


### exist_check
|  Value     |Result          |
|------------|----------------|
|true   |Check existing file before downloading|
|false  |Don't check existing file |



