#[macro_use]
extern crate rocket;
use rocket::{fairing::AdHoc, State};
use rspotify::{
    model::{AdditionalType, Country, FullTrack, Market, PlayableItem},
    prelude::OAuthClient,
    scopes, AuthCodeSpotify, Credentials, OAuth,
};
use std::io::Write;
use tokio::time::{interval_at, Instant};

struct SpotifyClient {
    client: AuthCodeSpotify,
    auth_code_rx: tokio::sync::mpsc::Receiver<String>,
}

impl SpotifyClient {
    fn new(auth_code_rx: tokio::sync::mpsc::Receiver<String>) -> Self {
        // Grab the credentials from the env file
        // We need RSPOTIFY_CLIENT_ID, RSPOTIFY_CLIENT_SECRET, RSPOTIFY_REDIRECT_URI defined there
        let creds = Credentials::from_env().unwrap();
        let oauth = OAuth::from_env(scopes!("user-read-currently-playing")).unwrap();
        let client = AuthCodeSpotify::new(creds, oauth);
        Self {
            client: client,
            auth_code_rx,
        }
    }

    // TODO: Make this return a Result, so we can handle errors more gracefully
    // TODO: Change the code to check if the token is already cached, and if so, use that instead of reauthing
    async fn auth_spotify(&mut self) {
        let spotify = &self.client;
        // Get the URL to authorise the app and pass it onto the cli prompt for the user to copy/paste
        let url = spotify.get_authorize_url(false).unwrap();
        // spotify.prompt_for_token(&url).await.unwrap();

        match webbrowser::open(url.as_str()) {
            Ok(_) => println!("Opened {} in your browser.", url),
            Err(why) => eprintln!(
                "Error when trying to open an URL in your browser: {:?}. \
                 Please navigate here manually: {}",
                why, url
            ),
        }

        // Wait for the webbrowser to get the code and then use it to get the token
        let code = self.auth_code_rx.recv().await.unwrap();
        println!("Got code in spotifyClient: {}", code);

        spotify
            .request_token(&code)
            .await
            .expect("Couldn't get token :(");

        // We should turn this into a result, so that if there's an issue and we can't get the token, then
        // we can more gracefully handle it instead of just assuming it's done unless an error is thrown
    }

    async fn get_currently_playing(&self) -> Option<FullTrack> {
        let spotify = &self.client;
        let market = Market::Country(Country::UnitedKingdom);
        // We only care about the Track type, so we can filter out the rest
        let additional_types = [AdditionalType::Track];

        // Get the currently playing track details
        // Notice that we unwrap this twice. Once to get the Option out of the Result, and then to get the actual response
        let spotify_response = spotify
            .current_playing(Some(market), Some(&additional_types))
            .await
            .expect("Some issue talking with Spotify API");

        match spotify_response {
            Some(spotify_response) => {
                // Slightly ugly part of Rust here. The item could be a Track or an Episode (in theory - we're filtering on Track above)
                // so we need to match the enum and grab the track out of the Enum
                if let PlayableItem::Track(track) = spotify_response.item.unwrap() {
                    return Some(track);
                }
            }
            None => return None,
        }
        None
    }

    async fn print_current_track_info(&self) {
        // Grab the track info from the Spotify API
        let track = self.get_currently_playing().await;

        // Clear the console
        print!("\x1B[2J\x1B[1;1H");
        // If that worked out ok, then print the track info
        match track {
            Some(track) => {
                // print the track
                print!(
                    "{} - {} ({})",
                    track.artists[0].name, track.name, track.popularity
                );
            }
            None => {
                print!("Nothing playing");
            }
        }

        // Flush to make sure it's printed
        std::io::stdout().flush().unwrap();
    }
}

#[get("/?<code>&<state>&<error>")]
async fn handle_auth_response(
    code: Option<String>,
    state: String,
    error: Option<String>,
    tx: &State<tokio::sync::mpsc::Sender<String>>,
) {
    if let Some(error) = error {
        println!("Error in auth response: {}", error);
        return ();
    }

    if let Some(code) = code {
        // We have the code, so send it across to the main thread, so we can continue the auth with Spotify
        tx.send(code).await.unwrap();
    }
}

#[rocket::main]
async fn main() {
    let (auth_code_tx, auth_code_rx) = tokio::sync::mpsc::channel(100);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

    let mut spotify_client = SpotifyClient::new(auth_code_rx);

    let config = rocket::Config {
        port: 3000,
        ..Default::default()
    };

    tokio::spawn(async move {
        // Wait for the web server to be ready
        ready_rx.await.unwrap();

        println!("Authing Spotify...");
        spotify_client.auth_spotify().await;

        // Now we're authed, we can start the loop to print the track info
        let mut interval = interval_at(Instant::now(), std::time::Duration::from_secs(5));

        loop {
            interval.tick().await;
            spotify_client.print_current_track_info().await;
        }
    });

    // Start the web server
    rocket::build()
        .configure(config)
        .mount("/", routes![handle_auth_response])
        .manage(auth_code_tx.clone())
        .attach(AdHoc::on_liftoff("Start the auth process", |_| {
            Box::pin(async move {
                ready_tx.send(()).unwrap();
            })
        }))
        .launch()
        .await
        .expect("Some issue with the web server");
}
