# Spotify What's On

Simple Spotify Terminal App made in Rust. This prints out the currently playing track on the user's spotify account, along with the popularity score, to the terminal.

This is intended as a mini coding project to play around with both Rust and the Spotify API.

## Compiling

To compile, you first need to make sure you have a `.env` file with the following:

| Var name               | Details                                       |
| ---------------------- | --------------------------------------------- |
| RSPOTIFY_CLIENT_ID     | The client ID of your spotify app             |
| RSPOTIFY_CLIENT_SECRET | The secret for your spotify app               |
| RSPOTIFY_REDIRECT_URI  | This should be set to `http://localhost:3000` |

These will require you to register the app with spotify and set the redirect to `http://localhost:3000`. Any changes to this will require updating the code,
so that Rocket is then listening on the correct URI.

Once done, just do the standard compilation:

```
cargo build
```

## Running

```
cargo run
```

To stop the application, use `ctrl-c`.

## To-do

[ ] Add graceful exit command

[ ] Add ability to get more information about playing song

[ ] Add more graceful way to display errors

[ ] Make the auth successful screen look better

[ ] Daemon mode???

[ ] Add testing!!! Everyone love's testing!
