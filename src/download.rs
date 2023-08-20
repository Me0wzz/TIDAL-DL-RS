use std::{
    cmp::min,
    default, fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::constants::*;
use base64::Engine;
use futures::{stream, Future, StreamExt};
use http::{HeaderMap, HeaderValue};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lofty::{Accessor, PictureType, Probe, Tag, TagExt, TaggedFileExt, TagType};
use reqwest::{Client, Url};
use serde_json::Value;
use tokio::task;

use crate::{
    constants::TIDAL_BASE,
    tidal_client::{remove_non_alphanumeric, TidalClient},
    UrlId, UrlType,
};

#[derive(Default, Debug, Clone)]
pub struct TrackInfo {
    title: String,
    album: String,
    artist: String,
    artists: String,
    cover_id: String,
    track_number: u32,
    track_id: u32,
    audio_quality: String,
}

pub async fn get_tracks_from_id(
    t_client: &TidalClient,
    r_client: Client,
    url_id: UrlId,
    url_type: &UrlType,
) -> (Vec<TrackInfo>, String) {
    let mut v: Vec<TrackInfo> = Vec::new();
    let params = [("countryCode", t_client.user_info.country_code.as_str()),
    ("limit", "10")];
    let id = match url_id {
        UrlId::Primary(url_id) => url_id.to_string(),
        UrlId::Playlist(url_id) => url_id,
    };
    let mut header = HeaderMap::new();
    let token = format!("Bearer {}", t_client.user_info.access_token);
    header.insert(
        "authorization",
        HeaderValue::from_str(token.as_str()).unwrap(),
    );
    
    let url = match url_type {
        UrlType::Track => format!("{}/tracks/{}", TIDAL_BASE, id),
        UrlType::Album => format!("{}/albums/{}/items", TIDAL_BASE, id),
        UrlType::Artist => todo!(),
        UrlType::Playlist => format!("{}/playlists/{}/items", TIDAL_BASE, id),
    };
    
    let urls = reqwest::Url::parse_with_params(&url, params).unwrap();
    let resp = r_client.get(urls).headers(header.clone()).send().await.unwrap();
    let result = resp.json::<serde_json::Value>().await.unwrap();
    let vec_value: Vec<Value> = vec![result.clone()];
    let tracks = if *url_type != UrlType::Track {
        result["items"].as_array().unwrap()
    } else {
        &vec_value
    };
   // println!("{}",tracks.len());
    for mut track in tracks {
        if *url_type != UrlType::Track {
            track = &track["item"];
        }
        let cover_id = track["album"]["cover"]
            .as_str()
            .unwrap()
            .to_string()
            .replace("-", "/");
        let album = track["album"]["title"].as_str().unwrap().to_string();
        let title = track["title"].as_str().unwrap().to_string();
        let track_id = track["id"].to_string().parse::<u32>().unwrap();
        let track_number = track["trackNumber"].to_string().parse::<u32>().unwrap();
        let artists = &track["artists"].as_array().unwrap();
        let artists: Vec<String> = artists
            .iter()
            .map(|v| v["name"].as_str().unwrap().to_string())
            .collect();
        let artists = artists.join(", ");
        let artist = track["artist"]["name"].as_str().unwrap().to_string();
        let audio_quality = track["audioQuality"].as_str().unwrap().to_string();

        v.push(TrackInfo {
            title,
            album,
            artists,
            artist,
            cover_id,
            track_number,
            track_id,
            audio_quality,
        })
    }let mut dl_path: String = t_client.config.download_path.clone();
    if *url_type == UrlType::Playlist {
        let url = reqwest::Url::parse_with_params(format!("{}/playlists/{}", TIDAL_BASE, id).as_str(), params).unwrap();
        let resp = r_client.get(url).headers(header).send().await.unwrap();
        let title = resp.json::<serde_json::Value>().await.unwrap().get("title").unwrap().as_str().unwrap().to_string();
        dl_path.push_str(format!("Playlist/{}",title).as_str());
    } else {
        dl_path.push_str(format!("Album/{}/{}", v[0].artist, v[0].album).as_str())
    }
    //println!("{}", v.len());
    (v, dl_path)
}

pub async fn download(t_client: &TidalClient, tracks: &Vec<TrackInfo>, url_type: &UrlType, dl_path: String) {
    let a = tracks.len();
    let params = [
        ("audioquality", "HI_RES"),
        ("playbackmode", "STREAM"),
        ("assetpresentation", "FULL"),
        ("limit", "50"),
    ];
    let mut header = HeaderMap::new();
    let token = format!("Bearer {}", t_client.user_info.access_token);
    header.insert(
        "authorization",
        HeaderValue::from_str(token.as_str()).unwrap(),
    );
    let mut urls = vec![];
    for i in 0..tracks.len() {
        let (url, file_name) = download_track(header.clone(), &params, tracks, i, &url_type).await;
        urls.push((url, file_name));
    }
    let client = Client::new();
    let m = MultiProgress::new();
    let bodies = futures::stream::iter(urls).enumerate()
        .map(|(i,(url, file_name))| {
            let c = tracks.to_vec();
            let client = client.clone();
            let pb = m.add(ProgressBar::new(0));
            let mut dl_path = dl_path.clone();
            tokio::spawn(async move {
                let resp = client.get(url.clone()).send().await.unwrap();
                pb.set_length(
                    resp.content_length()
                        .ok_or(format!("Failed to get content length from ''"))
                        .unwrap(),
                );
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{msg:.cyan} [{bytes}/{total_bytes}] [{elapsed_precise}] [{wide_bar:.green}] ({bytes_per_sec}, {eta})")
                        .unwrap()
                        .progress_chars("#>="),
                );
                
                let total_size = resp.content_length().unwrap();
                let mut stream = resp.bytes_stream();
                fs::create_dir_all(dl_path.clone()).unwrap();
                dl_path = format!("{}/{}",dl_path,file_name);
                pb.set_message(file_name);
                let mut file = std::fs::File::create(dl_path.clone())
                    .or(Err(format!("Failed to create file '")))
                    .unwrap();
                let mut downloaded: u64 = 0;
                while let Some(item) = stream.next().await {
                    let chunk = item
                        .or(Err(format!("Error while downloading file")))
                        .unwrap();
                    file.write_all(&chunk)
                        .or(Err(format!("Error while writing to file")))
                        .unwrap();
                    let new = min(downloaded + (chunk.len() as u64), total_size);
                    downloaded = new;
                    pb.set_position(new);
                }
                pb.set_message("Writing ID3");
                write_metadata(&c[i], client, dl_path).await;
                pb.finish_with_message(format!("Downloaded"));
            })
        })
        .buffer_unordered(tracks.len());
    bodies
        .for_each(|b| async {
            match b {
                Ok(o) => {}
                Err(e) => eprintln!("Got a tokio::JoinError: {}", e),
            }
        })
        .await;

    /*
    let mut downloads = vec![];

    let mut path = String::new();
    for i in 0..tracks.len() {
        let (a, f_n, path_tmp) = download_track(header.clone(), &params, tracks, i).await;
        path = path_tmp;
        let url = Url::from_str(a.as_str()).unwrap();
        downloads.push(trauma::download::Download::new(&url, &f_n))
    }
    let downloader = DownloaderBuilder::new()
        .directory(PathBuf::from(path))
        .build();
    downloader.download(&downloads).await;
    */
}

pub async fn write_metadata(track: &TrackInfo, request: Client, path_str: String) {
    let cover_url = format!(
        "https://resources.tidal.com/images/{}/1280x1280.jpg",
        track.cover_id
    );
    //eprintln!("{:?}", track);
    let cover = request
        .get(cover_url)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    //head > title
        //println!("tag: {}", file_name);
    let path = Path::new(path_str.as_str());
    //eprintln!("{}", path_str);
    let mut tagged_file = Probe::open(&path).unwrap().read().unwrap();
    let tag = match tagged_file.primary_tag_mut() {
        Some(primary_tag) => {
            //eprintln!("{}:{:?}", path_str, primary_tag.tag_type());
            primary_tag
        },
        None => {
            if let Some(first_tag) = tagged_file.first_tag_mut() {
                //println!("{}:{:?}", path_str, first_tag.tag_type());
                first_tag
            } else {
                let tag_type = tagged_file.primary_tag_type();
                //eprintln!("WARN: No tags found, creating a new tag of type `{tag_type:?}`");
                tagged_file.insert_tag(Tag::new(tag_type));

                tagged_file.primary_tag_mut().unwrap()
            }
        }
    };
    
    let title = &track.title;
    let artists = &track.artists;
    let picture = lofty::Picture::new_unchecked(
        PictureType::CoverFront,
        lofty::MimeType::Jpeg,
        None,
        cover.to_vec(),
    );
    let tracknumber = track.track_number;
    let album = &track.album;
    tag.set_picture(0, picture);
    tag.set_title(title.to_string());
    tag.set_artist(artists.to_string());
    tag.set_track(tracknumber);
    tag.set_album(album.to_string());

    tag.save_to_path(&path).unwrap();
}

pub async fn download_track<'a>(
    header: HeaderMap,
    param: &'a [(&'a str, &'a str)],
    tracks: &'a Vec<TrackInfo>,
    index: usize,
    url_type: &UrlType
) -> (String, String) {
    let header_arc = Arc::new(Mutex::new(header));
    let len = tracks.len();
    let header_copy = Arc::clone(&header_arc);
    let header = header_copy.lock().unwrap().clone();

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .unwrap();

    let url = format!(
        "{}/tracks/{}/playbackinfopostpaywall",
        TIDAL_BASE, tracks[index].track_id
    );
    //println!("url: {url}");
    let urls = reqwest::Url::parse_with_params(&url, param).unwrap();
    let resp = client
        .get(urls)
        .headers(header.clone())
        .send()
        .await
        .unwrap();
    //println!("{resp:?}");
    //println!("{}", resp.status());
    let result = resp.json::<serde_json::Value>().await.unwrap();
    
    //println!("result: {}", result);
    let manifest = remove_non_alphanumeric(result.get("manifest").unwrap().to_string());

    let d_manifest =
        String::from_utf8(base64::prelude::BASE64_STANDARD.decode(manifest).unwrap()).unwrap();
    let d_manifest_json: Value = serde_json::from_str(&d_manifest).unwrap();
    let d_url = d_manifest_json.get("urls").unwrap().to_string();
    let d_url = remove_non_alphanumeric(d_url);
    let mut file_name = format!("{} - {}.flac", tracks[index].artist, tracks[index].title);

    if tracks[index].audio_quality.contains("HIGH") {
        file_name = format!("{} - {}.mp4", tracks[index].artist, tracks[index].title)
    };
    let path = match url_type {
        UrlType::Playlist => format!("Playlist/{}/{}", tracks[index].artist, tracks[index].album),
        _ => format!("Album/{}/{}", tracks[index].artist, tracks[index].album)
    };

    (d_url, file_name)
    /*

    let d_file = 
        .get(d_url)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let mut file_name = format!(
        "Album/{}/{}/{} - {}.flac",
        tracks[index].artist, tracks[index].album, tracks[index].artist, tracks[index].title
    );

    if tracks[index].audio_quality.contains("HIGH") {
        file_name = format!(
            "Album/{}/{}/{} - {}.m4a",
            tracks[index].artist, tracks[index].album, tracks[index].artist, tracks[index].title
        )
    };

    if Path::new(file_name.as_str()).exists() {
        println!(
            "File: {} - {}.flac already exists! skip downloading..",
            tracks[index].artist, tracks[index].title
        );
        return;
    }
    let mut file = fs::File::create(file_name.clone()).unwrap();
    file.write(&d_file).unwrap();
    write_metadata(&tracks[index], client).await;
    println!("Downloaded: {file_name}");
    */
}
