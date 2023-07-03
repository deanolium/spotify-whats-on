#[macro_use]
extern crate rocket;
use rocket::{fairing::AdHoc, State};
use rspotify::model::{AudioFeatures, Modality};
use std::io::Write;
use tokio::time::{interval_at, Instant};
mod spotify_api;

fn clear_console() {
    print!("\x1B[2J\x1B[1;1H");
}

fn get_key(key_index: i32) -> String {
    let possible_keys = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];

    let key = if key_index == -1 {
        "Not Detected"
    } else {
        *possible_keys.get(key_index as usize).unwrap()
    };

    String::from(key)
}

fn print_track_features(features: AudioFeatures) {
    let mode = if features.mode == Modality::Major {
        "Major"
    } else {
        "Minor"
    };

    let key = get_key(features.key);

    print!(
        "\n\nAcousticness: {}
Danceability: {}
Energy: {}
Intrumentalness: {}
Key: {}
Liveness: {}
Loudness: {}
Mode: {}
Speechiness: {}
Tempo: {}
Time Signature: {}/4
Valence: {}",
        features.acousticness,
        features.danceability,
        features.energy,
        features.instrumentalness,
        key,
        features.liveness,
        features.loudness,
        mode,
        features.speechiness,
        features.tempo,
        features.time_signature,
        features.valence
    );
}

// Print out the currently playing track on the provided spotify client
async fn print_current_track_info(spotify_client: &mut spotify_api::SpotifyClient) {
    // Grab the track info from the Spotify API
    let track = spotify_client.get_currently_playing().await;

    // Clear the console
    clear_console();

    // If that worked out ok, then print the track info
    match track {
        Some(track) => {
            // print the track
            print!(
                "{} - {} ({})",
                track.artists[0].name, track.name, track.popularity
            );

            if let Some(track_id) = track.id {
                let features = spotify_client.get_track_features(track_id).await;

                if let Some(features) = features {
                    print_track_features(features);
                }
            }
        }
        None => {
            print!("Nothing playing");
        }
    }

    // Flush to make sure it's printed
    std::io::stdout().flush().unwrap();
}

fn print_error(reason: &str, error: &str) {
    clear_console();
    println!("**{}**", reason);
    println!("**Error: {}", error);
}

#[get("/?<code>&<error>")]
async fn handle_auth_response(
    code: Option<String>,
    error: Option<String>,
    tx: &State<tokio::sync::mpsc::Sender<String>>,
) -> &'static str {
    if let Some(error) = error {
        println!("Error in auth response: {}", error);
        return "Error in auth response!";
    }

    if let Some(code) = code {
        // We have the code, so send it across to the main thread, so we can continue the auth with Spotify
        tx.send(code).await.unwrap();
    }

    "Thanks for authorising the app. You can close this window now."
}

// The main thread for the application
// This uses ready_rx to wait for the web server to say it's ready before setting up the spotify client
// and running the main loop
// auth_code_rx is used to receive the authentication code from Spotify if it needs to get the user to login
// and then accesses the callback
async fn main_loop_thread(
    ready_rx: tokio::sync::oneshot::Receiver<()>,
    auth_code_rx: tokio::sync::mpsc::Receiver<String>,
) {
    // Wait for the web server to be ready
    ready_rx.await.unwrap();

    let mut spotify_client = spotify_api::SpotifyClient::new(auth_code_rx);

    match spotify_client.auth_spotify().await {
        Ok(_) => {
            let mut interval = interval_at(Instant::now(), std::time::Duration::from_secs(5));

            loop {
                interval.tick().await;
                print_current_track_info(&mut spotify_client).await;
            }
        }

        Err(error) => {
            print_error("Token Error", error.as_str());
        }
    }
}

#[rocket::main]
async fn main() {
    // A channel to handle passing the response code from the Spotify API to the main loop thread
    let (auth_code_tx, auth_code_rx) = tokio::sync::mpsc::channel::<String>(100);

    // A channel to say when the web server is online and ready for us to actually do stuff
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

    // The main Spotify Loop put into its own thread
    tokio::spawn(main_loop_thread(ready_rx, auth_code_rx));

    let server_config = rocket::Config {
        port: 3000,
        ..Default::default()
    };

    // Start the web server
    // This is purely to handle the Spotify auth api returning on successful auth
    rocket::build()
        .configure(server_config)
        .mount("/", routes![handle_auth_response])
        .manage(auth_code_tx.clone())
        .attach(AdHoc::on_liftoff("Flag to start the auth process", |_| {
            Box::pin(async move {
                ready_tx.send(()).unwrap();
            })
        }))
        .launch()
        .await
        .expect("Some issue with the web server");
}
