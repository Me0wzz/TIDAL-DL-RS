mod constants;
mod download;
mod download_parallel;
mod tidal_client;

use std::env;

use crate::constants::*;
use crate::tidal_client::*;
use download::*;

#[derive(PartialEq)]
pub enum UrlType {
    Track,
    Album,
    Artist,
    Playlist,
}

pub enum UrlId {
    Primary(u32),
    Playlist(String),
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() > 1);

    let token = get_token().await;
    let request = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .unwrap();
    let mut client = TidalClient::new(token);
    client.get_device_code("zU4XHVVkc2tDPo4t".to_string()).await;
    client.login_session().await;
    client.save_token().await;
    for i in 1..args.len() {
        let (id, url_type) = parse_url(args.get(i).unwrap());
        let tracks = get_tracks_from_id(&client, request.clone(), id, &url_type).await;
        download(&client, &tracks.0, &url_type, tracks.1).await;
    }

    Ok(())
}

fn parse_url(url: &str) -> (UrlId, UrlType) {
    if !url.clone().contains("tidal.com") {
        panic!("invalid url ");
    }
    let (_, id) = url.rsplit_once("/").unwrap();
    let album_id_option = id.parse::<u32>();

    let mut url_type = UrlType::Track;
    let url_id = match album_id_option {
        Ok(some) => UrlId::Primary(some),
        Err(_) => {
            url_type = UrlType::Artist;
            UrlId::Playlist(id.to_string())
        }
    };
    if url.contains("album") {
        url_type = UrlType::Album;
    }
    if url.contains("playlist") {
        url_type = UrlType::Playlist;
    }

    (url_id, url_type)
}
