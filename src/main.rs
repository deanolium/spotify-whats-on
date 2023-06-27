use rspotify::{
    model::{AdditionalType, Country, FullTrack, Market, PlayableItem},
    prelude::OAuthClient,
    scopes, AuthCodeSpotify, Credentials, OAuth,
};
use std::io::Write;
use tokio::time::{interval_at, Instant};

// Authorise Spotify on the CLI
// Note: This will open a browser window and require the user to copy/paste the return URI into the CLI
// This is a limitation of the rspotify library
// Ideally we should change this to handle the proper flow, but that requires running a local server which
// the Spotify Auth API can redirect to, and then we need to do what we want... it's a lot of work!
async fn auth_spotify() -> AuthCodeSpotify {
    // Grab the credentials from the env file
    // We need RSPOTIFY_CLIENT_ID, RSPOTIFY_CLIENT_SECRET, RSPOTIFY_REDIRECT_URI defined there
    let creds = Credentials::from_env().unwrap();
    let oauth = OAuth::from_env(scopes!("user-read-currently-playing")).unwrap();
    let spotify = AuthCodeSpotify::new(creds, oauth);

    // Get the URL to authorise the app and pass it onto the cli prompt for the user to copy/paste
    let url = spotify.get_authorize_url(false).unwrap();
    spotify.prompt_for_token(&url).await.unwrap();

    return spotify;
}

// Get the currently playing track details
async fn get_currently_playing(spotify: &AuthCodeSpotify) -> Option<FullTrack> {
    let market = Market::Country(Country::UnitedKingdom);
    // We only care about the Track type, so we can filter out the rest
    let additional_types = [AdditionalType::Track];

    // Get the currently playing track details
    // Notice that we unwrap this twice. Once to get the Option out of the Result, and then to get the actual response
    // TODO: Handle the potential errors properly!
    let spotify_response = spotify
        .current_playing(Some(market), Some(&additional_types))
        .await
        .unwrap()
        .unwrap();

    // Slightly ugly part of Rust here. The item could be a Track or an Episode (in theory - we're filtering on Track above)
    // so we need to match the enum and grab the track out of the Enum
    if let PlayableItem::Track(track) = spotify_response.item.unwrap() {
        return Some(track);
    }
    return None;
}

// Print the currently playing track details to the console
async fn print_current_track_info(spotify: &AuthCodeSpotify) {
    // Grab the track info from the Spotify API
    let track = get_currently_playing(&spotify).await;

    // If that worked out ok, then print the track info
    if let Some(track) = track {
        // Clear the console and print the track
        print!("\x1B[2J\x1B[1;1H");
        print!(
            "{} - {} ({})",
            track.artists[0].name, track.name, track.popularity
        );
        // Flush to make sure it's printed
        std::io::stdout().flush().unwrap();
    }
}

#[tokio::main]
async fn main() {
    let spotify = auth_spotify().await;

    // Set up a basic interval timer to update the track info every 5 seconds
    let mut interval = interval_at(Instant::now(), std::time::Duration::from_secs(5));

    // Keep going forever
    // TODO: Make this able to handle some keyboard controls, so we can quit or ask for different info
    loop {
        interval.tick().await;
        print_current_track_info(&spotify).await;
    }
}
