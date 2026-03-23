use anyhow::Result;
use clap::{Parser, Subcommand};
use opencast_core::MediaInfo;
use opencast_discovery::SsdpDiscovery;
use opencast_dlna::DlnaController;
use std::time::Duration;


#[derive(Parser)]
#[command(name = "opencast", about = "OpenCast - Open Source Media Casting CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover DLNA media renderers on the local network
    Discover {
        /// Search timeout in seconds
        #[arg(short, long, default_value_t = 5)]
        timeout: u64,
    },

    /// Cast a media URL to a DLNA renderer
    Cast {
        /// Media URL to cast
        url: String,

        /// Target device name (partial match)
        #[arg(short, long)]
        device: Option<String>,

        /// Media title
        #[arg(short, long)]
        title: Option<String>,

        /// MIME type (auto-detected if not specified)
        #[arg(short, long)]
        mime: Option<String>,

        /// Discovery timeout in seconds
        #[arg(long, default_value_t = 5)]
        timeout: u64,
    },

    /// Control playback on a DLNA renderer
    Control {
        /// Action: play, pause, stop, seek
        action: String,

        /// Target device name (partial match)
        #[arg(short, long)]
        device: Option<String>,

        /// Seek position in seconds (for seek action)
        #[arg(short, long)]
        position: Option<f64>,

        /// Volume level 0-100 (for volume action)
        #[arg(short, long)]
        volume: Option<u32>,

        /// Discovery timeout in seconds
        #[arg(long, default_value_t = 5)]
        timeout: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Discover { timeout } => {
            println!("Searching for DLNA devices...\n");
            let discovery = SsdpDiscovery::new();
            let devices = discovery
                .search_renderers(Duration::from_secs(timeout))
                .await?;

            if devices.is_empty() {
                println!("No devices found. Make sure you're on the same network.");
            } else {
                println!("Found {} device(s):\n", devices.len());
                for (i, dev) in devices.iter().enumerate() {
                    println!(
                        "  {}. {} ({})",
                        i + 1,
                        dev.name,
                        dev.location
                    );
                    if let Some(ref mfg) = dev.manufacturer {
                        println!("     Manufacturer: {mfg}");
                    }
                    if let Some(ref model) = dev.model_name {
                        println!("     Model: {model}");
                    }
                    println!();
                }
            }
        }

        Commands::Cast {
            url,
            device,
            title,
            mime,
            timeout,
        } => {
            let discovery = SsdpDiscovery::new();
            let devices = discovery
                .search_renderers(Duration::from_secs(timeout))
                .await?;

            let target = find_device(&devices, device.as_deref())?;

            let mime_type = mime.unwrap_or_else(|| guess_mime_type(&url));
            let media = MediaInfo::new(&url)
                .with_title(title.unwrap_or_else(|| "OpenCast Media".to_string()))
                .with_mime_type(mime_type);

            let controller = DlnaController::new(target.clone())?;
            controller.cast(&media).await?;

            println!("Casting to '{}': {}", target.name, url);
            println!("Your device is now free to use!");
        }

        Commands::Control {
            action,
            device,
            position,
            volume,
            timeout,
        } => {
            let discovery = SsdpDiscovery::new();
            let devices = discovery
                .search_renderers(Duration::from_secs(timeout))
                .await?;

            let target = find_device(&devices, device.as_deref())?;
            let controller = DlnaController::new(target.clone())?;

            match action.as_str() {
                "play" => {
                    controller.play().await?;
                    println!("Resumed playback on '{}'", target.name);
                }
                "pause" => {
                    controller.pause().await?;
                    println!("Paused playback on '{}'", target.name);
                }
                "stop" => {
                    controller.stop().await?;
                    println!("Stopped playback on '{}'", target.name);
                }
                "seek" => {
                    let pos = position.unwrap_or(0.0);
                    controller.seek(pos).await?;
                    println!("Seeked to {pos:.0}s on '{}'", target.name);
                }
                "volume" => {
                    let vol = volume.unwrap_or(50);
                    controller.set_volume(vol).await?;
                    println!("Set volume to {vol}% on '{}'", target.name);
                }
                "status" => {
                    let state = controller.get_transport_state().await?;
                    let pos = controller.get_position_info().await?;
                    let vol = controller.get_volume().await?;
                    println!("Device: {}", target.name);
                    println!("State:  {state:?}");
                    println!(
                        "Position: {:.0}s / {:.0}s",
                        pos.position, pos.duration
                    );
                    if let Some(uri) = &pos.track_uri {
                        println!("URI:    {uri}");
                    }
                    println!(
                        "Volume: {:.0}% (muted: {})",
                        vol.level * 100.0,
                        vol.muted
                    );
                }
                other => {
                    anyhow::bail!(
                        "Unknown action: {other}. Use: play, pause, stop, seek, volume, status"
                    );
                }
            }
        }
    }

    Ok(())
}

fn find_device(
    devices: &[opencast_core::Device],
    name_filter: Option<&str>,
) -> Result<opencast_core::Device> {
    if devices.is_empty() {
        anyhow::bail!("No DLNA devices found on the network.");
    }

    match name_filter {
        Some(filter) => {
            let filter_lower = filter.to_lowercase();
            devices
                .iter()
                .find(|d| d.name.to_lowercase().contains(&filter_lower))
                .cloned()
                .ok_or_else(|| {
                    let names: Vec<_> = devices.iter().map(|d| d.name.as_str()).collect();
                    anyhow::anyhow!(
                        "No device matching '{}'. Available: {}",
                        filter,
                        names.join(", ")
                    )
                })
        }
        None => {
            // Use first device found
            Ok(devices[0].clone())
        }
    }
}

fn guess_mime_type(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.ends_with(".mp4") || lower.ends_with(".m4v") {
        "video/mp4"
    } else if lower.ends_with(".mkv") {
        "video/x-matroska"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".avi") {
        "video/avi"
    } else if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".flac") {
        "audio/flac"
    } else if lower.ends_with(".m3u8") {
        "application/vnd.apple.mpegurl"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".png") {
        "image/png"
    } else {
        "video/mp4"
    }
    .to_string()
}
