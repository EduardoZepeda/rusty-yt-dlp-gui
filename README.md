# rusty-yt-dlp-gui

A Linux lightweight, multilingual, minimalist GUI application for downloading YouTube videos and audio, built with Rust. ~~Blazing fast, just kidding~~

![Rusty GUI YT-DLP wrapper](https://res.cloudinary.com/dwrscezd2/image/upload/v1751322441/coffee-bytes/rusty-yt-dlp-gui_orsijk.webp)

## Motivation

This app was **half-vibe-coded half-coded to test the capabilities of Windsurf browser**. Hence you can expect a little bit of chaos here and there. I tried to refactor the code as much as possible (not really) to get as close as possible to a more maintainable codebase but since this is a small test project it's not a priority.

## Features

- 🚀 Simple and intuitive interface
- ⚡ Built with Rust for performance and reliability
- 📦 Self-contained, no system-wide yt-dlp installation required
- 🎥 Download videos in MP4 format
- 🎵 Download audio in MP3 format
- 🌍 Multi-language support (english and spanish for now)

## Requirements

- Rust (latest stable version recommended)
- Internet connection

## Installation

1. Clone this repository
2. Build the project:
   ```
   cargo build --release
   ```
3. Run the application:
   ```
   // Just in case
   chmod +x ./target/release/ytdl-gui
   ./target/release/ytdl-gui
   ```

## Usage

1. Enter a YouTube URL
2. Select your preferred format (MP3 or MP4)
3. Choose a download directory
4. Click "Download"

## License

This project is licensed under the [MIT License](LICENSE).