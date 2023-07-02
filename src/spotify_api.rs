use rspotify::{
    model::{AdditionalType, Country, FullTrack, Market, PlayableItem},
    prelude::OAuthClient,
    scopes, AuthCodeSpotify, Config, Credentials, OAuth,
};

pub struct SpotifyClient {
    client: AuthCodeSpotify,
    auth_code_rx: tokio::sync::mpsc::Receiver<String>,
}

impl SpotifyClient {
    pub fn new(auth_code_rx: tokio::sync::mpsc::Receiver<String>) -> Self {
        // Grab the credentials from the env file
        // We need RSPOTIFY_CLIENT_ID, RSPOTIFY_CLIENT_SECRET, RSPOTIFY_REDIRECT_URI defined there
        let creds = Credentials::from_env().unwrap();
        let oauth = OAuth::from_env(scopes!("user-read-currently-playing")).unwrap();
        let config = Config {
            token_cached: true,
            token_refreshing: true,
            ..Default::default()
        };
        let client = AuthCodeSpotify::with_config(creds, oauth, config);

        Self {
            client,
            auth_code_rx,
        }
    }

    async fn check_if_valid_token_cached(&mut self) -> bool {
        if let Ok(Some(token)) = self.client.read_token_cache(false).await {
            *self.client.token.lock().await.unwrap() = Some(token);
            return true;
        }
        false
    }

    pub async fn auth_spotify(&mut self) -> Result<(), String> {
        if self.check_if_valid_token_cached().await {
            return Ok(());
        }

        // Get the URL to authorise the app and pass it onto the cli prompt for the user to copy/paste
        let url = self.client.get_authorize_url(false).unwrap();

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

        self.client
            .request_token(&code)
            .await
            .expect("Couldn't get token :(");

        Ok(())
    }

    pub async fn get_currently_playing(&mut self) -> Option<FullTrack> {
        let market = Market::Country(Country::UnitedKingdom);
        // We only care about the Track type, so we can filter out the rest
        let additional_types = [AdditionalType::Track];

        // Get the currently playing track details
        // Notice that we unwrap this twice. Once to get the Option out of the Result, and then to get the actual response
        let result = self
            .client
            .current_playing(Some(market), Some(&additional_types))
            .await;

        let spotify_response = match result {
            Ok(result) => result,
            Err(_) => {
                // first try auth again
                self.auth_spotify().await.expect("Error in re-authorizing");
                self.client
                    .current_playing(Some(market), Some(&additional_types))
                    .await
                    .expect("Second attempt at getting currently playing failed")
            }
        };

        if let Some(spotify_response) = spotify_response {
            // Slightly ugly part of Rust here. The item could be a Track or an Episode (in theory - we're filtering on Track above)
            // so we need to match the enum and grab the track out of the Enum
            if let PlayableItem::Track(track) = spotify_response.item.unwrap() {
                return Some(track);
            }
        }

        None
    }
}
